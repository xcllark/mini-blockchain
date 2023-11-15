use crate::{utils, DatabaseReader, DatabaseWriter};
use alloy_primitives::{Address, B256, U256};
use elliptic_curve::{consts::U32, sec1::ToEncodedPoint};
use k256::{
    ecdsa::{RecoveryId, Signature, VerifyingKey},
    elliptic_curve::generic_array::GenericArray,
    PublicKey,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::slice::{Iter, IterMut};
use std::vec::IntoIter;
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::RwLockReadGuard;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Transaction {
    /// Hash of the transaction
    pub hash: B256,
    /// From address
    pub from: Address,
    /// To address
    pub to: Address,
    /// Nonce of the transaction
    pub nonce: u64,
    /// Amount of coins to send
    pub value: u128,
    /// ECDSA recovery id
    pub v: u8,
    /// ECDSA signature r
    pub r: U256,
    /// ECDSA signature s
    pub s: U256,
}

impl Transaction {
    pub fn hash(&self) -> B256 {
        let mut hasher = Sha3::v256();
        hasher.update(&self.from[..]);
        hasher.update(&self.to[..]);
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.value.to_le_bytes());
        let mut buf = [0u8; 32];
        hasher.finalize(&mut buf);
        B256::from_slice(&buf)
    }

    pub fn get_hash(&self) -> B256 {
        self.hash
    }

    pub fn verify(&self) -> bool {
        let hash = self.hash();
        if hash != self.hash {
            return false;
        }

        let recovery_id = match RecoveryId::from_byte(self.v) {
            Some(id) => id,
            None => return false,
        };

        let signature = {
            let r_bytes = self.r.to_be_bytes::<32>();
            let s_bytes = self.s.to_be_bytes::<32>();
            let gar: &GenericArray<u8, U32> = GenericArray::from_slice(&r_bytes);
            let gas: &GenericArray<u8, U32> = GenericArray::from_slice(&s_bytes);
            match Signature::from_scalars(*gar, *gas) {
                Ok(sig) => sig,
                Err(_) => return false,
            }
        };

        let verify_key =
            match VerifyingKey::recover_from_prehash(&hash[..], &signature, recovery_id) {
                Ok(key) => key,
                Err(_) => return false,
            };

        let public_key = PublicKey::from(&verify_key);
        let public_key = public_key.to_encoded_point(false);
        let public_key = public_key.as_bytes();

        if public_key[0] != 0x04 {
            return false;
        }

        let hash = utils::sha3(&public_key[1..]);
        let addr = Address::from_word(hash);

        if addr != self.from {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BlockHeader {
    /// Hash of the parent block
    pub parent_hash: B256,
    /// Proof of work nonce
    pub nonce: u64,
    /// Number of the block
    pub number: u64,
    /// Timestamp of the block
    pub timestamp: u64,
    /// Difficulty of the block
    pub difficulty: U256,
    /// Address of the executor - Coinbase
    pub coinbase: Address,
    /// Merkle root of all the transactions included in this block
    pub tx_root: B256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// All transactions included in the block stored by their hash
    pub transactions: Transactions,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Transactions) -> Self {
        Self {
            header,
            transactions,
        }
    }

    pub fn hash(&self) -> B256 {
        let mut hasher = Sha3::v256();
        hasher.update(self.header.parent_hash.as_slice());
        hasher.update(&self.header.nonce.to_le_bytes());
        hasher.update(&self.header.number.to_le_bytes());
        hasher.update(&self.header.timestamp.to_le_bytes());
        hasher.update(self.header.difficulty.as_le_slice());
        hasher.update(&self.header.coinbase[..]);
        hasher.update(self.header.tx_root.as_slice());

        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        B256::from_slice(&hash)
    }
    pub fn seal_slow(self) -> SealedBlock {
        let header = SealedHeader {
            parent_hash: self.header.parent_hash,
            block_hash: self.hash(),
            number: self.header.number,
            nonce: self.header.nonce,
            timestamp: self.header.timestamp,
            difficulty: self.header.difficulty,
            coinbase: self.header.coinbase,
            tx_root: self.header.tx_root,
        };

        SealedBlock {
            header,
            transactions: self.transactions,
        }
    }

    pub fn seal(self, hash: B256) -> SealedBlock {
        let header = SealedHeader {
            parent_hash: self.header.parent_hash,
            block_hash: hash,
            nonce: self.header.nonce,
            difficulty: self.header.difficulty,
            number: self.header.number,
            timestamp: self.header.timestamp,
            coinbase: self.header.coinbase,
            tx_root: self.header.tx_root,
        };

        SealedBlock {
            header,
            transactions: self.transactions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SealedHeader {
    /// Hash of the parent block
    parent_hash: B256,

    /// Number of the block
    number: u64,

    /// Hash of the block
    block_hash: B256,

    /// Proof of work nonce
    nonce: u64,

    /// Difficulty of the block
    difficulty: U256,

    /// Timestamp of the block
    timestamp: u64,

    /// Address of the executor - Coinbase
    coinbase: Address,

    /// Merkle root of all the transactions in the block
    tx_root: B256,
}

/// # Sealed Block
/// Sealed block includes the hash of the entire block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SealedBlock {
    /// Sealed header
    header: SealedHeader,

    /// All transactions in the block
    transactions: Transactions,
}

impl SealedBlock {
    pub fn hash(&self) -> B256 {
        let mut hasher = Sha3::v256();
        hasher.update(self.header.parent_hash.as_slice());
        hasher.update(&self.header.nonce.to_le_bytes());
        hasher.update(&self.header.number.to_le_bytes());
        hasher.update(&self.header.timestamp.to_le_bytes());
        hasher.update(self.header.difficulty.as_le_slice());
        hasher.update(&self.header.coinbase[..]);
        hasher.update(self.header.tx_root.as_slice());

        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        B256::from_slice(&hash)
    }

    pub fn get_hash(&self) -> &B256 {
        &self.header.block_hash
    }

    /// Verify if the block is valid
    pub fn verify(&self) -> bool {
        let hash = self.hash();
        let u256_hash = U256::from_le_slice(&hash[..]);

        for tx in &self.transactions {
            if !tx.verify() || tx.hash() != tx.get_hash() {
                return false;
            }
        }

        hash == self.header.block_hash && u256_hash <= *self.difficulty()
    }

    pub fn difficulty(&self) -> &U256 {
        &self.header.difficulty
    }

    pub fn number(&self) -> u64 {
        self.header.number
    }

    pub fn transactions(&self) -> &Transactions {
        &self.transactions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Account {
    balance: u128,
    nonce: u64,
}

impl Account {
    pub fn new(balance: u128, nonce: u64) -> Self {
        Self { balance, nonce }
    }

    pub fn balance(&self) -> u128 {
        self.balance
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    pub fn update_balance(&mut self, new_balance: u128) {
        self.balance = new_balance;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Transactions {
    inner: Vec<Transaction>,
}

impl Transactions {
    pub fn get_root(&self) -> B256 {
        // TODO: In the future use a merkle tree
        let mut hasher = Sha3::v256();
        for tx in self {
            let tx_hash = tx.hash;
            hasher.update(&tx_hash[..]);
        }

        let mut buf = [0u8; 32];
        hasher.finalize(&mut buf);
        B256::from_slice(&buf)
    }

    pub fn push(&mut self, tx: Transaction) {
        self.inner.push(tx);
    }

    pub fn remove(&mut self, index: usize) -> Transaction {
        self.inner.remove(index)
    }

    /// We implement sort to make sure we process transactions from the same address
    /// sequentiall from lowest nonce to highest
    pub fn sort(&mut self) {
        self.inner.sort_by(|a, b| match a.from.cmp(&b.from) {
            std::cmp::Ordering::Equal => a.nonce.cmp(&b.nonce),
            other => other,
        })
    }
}

impl From<Vec<Transaction>> for Transactions {
    fn from(value: Vec<Transaction>) -> Self {
        Transactions { inner: value }
    }
}

impl IntoIterator for Transactions {
    type IntoIter = IntoIter<Transaction>;
    type Item = Transaction;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a Transactions {
    type IntoIter = Iter<'a, Transaction>;
    type Item = &'a Transaction;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a mut Transactions {
    type IntoIter = IterMut<'a, Transaction>;
    type Item = &'a mut Transaction;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TransactionReceipt {
    pub success: bool,
    pub block_hash: B256,
    pub block_number: u64,
    pub from: Address,
    pub to: Address,
}

impl TransactionReceipt {
    pub fn build(tx: &Transaction, block: &SealedBlock) -> Self {
        Self {
            success: false,
            block_hash: *block.get_hash(),
            block_number: block.number(),
            from: tx.from,
            to: tx.to,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChangeSet {
    pub touched_accounts: HashMap<Address, Account>,
    pub receipts: HashMap<B256, TransactionReceipt>,
}

impl ChangeSet {
    pub fn insert_receipt(&mut self, hash: &B256, receipt: TransactionReceipt) {
        self.receipts.insert(*hash, receipt);
    }

    pub fn insert_account(&mut self, addr: Address, account: Account) {
        self.touched_accounts.insert(addr, account);
    }

    pub fn get_account(&self, addr: &Address) -> Option<&Account> {
        self.touched_accounts.get(addr)
    }

    pub fn touched_accounts_ref(&self) -> &HashMap<Address, Account> {
        &self.touched_accounts
    }
}

pub struct State<'a, DB> {
    changeset: ChangeSet,
    db: &'a RwLockReadGuard<'a, DB>,
}

impl<'a, DB> State<'a, DB>
where
    DB: DatabaseWriter + DatabaseReader,
{
    pub fn new(db: &'a RwLockReadGuard<'a, DB>) -> Self {
        Self {
            changeset: ChangeSet::default(),
            db,
        }
    }

    pub fn get_account(&self, addr: &Address) -> Option<&Account> {
        // We first check if the account is in the change set to make
        // sure we are getting the latest data
        match self.changeset.touched_accounts_ref().get(addr) {
            Some(acc) => Some(acc),
            None => self.db.read_account(addr),
        }
    }

    pub fn insert_account(&mut self, addr: &Address, account: Account) {
        self.changeset.insert_account(*addr, account);
    }

    pub fn insert_receipt(&mut self, tx_hash: &B256, tx_receipt: TransactionReceipt) {
        self.changeset.insert_receipt(tx_hash, tx_receipt)
    }
}

impl<'a, DB> From<State<'a, DB>> for ChangeSet {
    fn from(value: State<'a, DB>) -> Self {
        value.changeset
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{addr, sign_hash, u256_to_signing_key};

    use super::*;

    #[test]
    fn test_verify() {
        let pk = U256::from(98234);
        let pk = u256_to_signing_key(&pk).unwrap();

        let mut tx = Transaction::default();

        let addr = addr(&pk);
        tx.from = addr;

        tx.hash = tx.hash();

        let (v, r, s) = sign_hash(tx.hash(), &pk);

        tx.v = v;
        tx.r = r;
        tx.s = s;

        assert!(tx.verify());
    }

    #[test]
    fn test_verify_block() {
        let pk = U256::from(98234);
        let pk = u256_to_signing_key(&pk).unwrap();

        let mut tx = Transaction::default();

        let addr = addr(&pk);
        tx.from = addr;

        tx.hash = tx.hash();

        let (v, r, s) = sign_hash(tx.hash(), &pk);

        tx.v = v;
        tx.r = r;
        tx.s = s;

        let mut block = Block {
            header: BlockHeader {
                parent_hash: B256::ZERO,
                nonce: 0,
                number: 0,
                timestamp: 0,
                difficulty: U256::MAX,
                coinbase: Address::ZERO,
                tx_root: B256::ZERO,
            },
            transactions: Transactions::default(),
        };

        block.transactions.inner.push(tx);

        let sealed_block = block.seal_slow();
        assert!(sealed_block.verify());
    }
}
