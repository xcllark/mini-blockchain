use crate::{Account, ChainSpec, Error, SealedBlock, Transaction, TransactionReceipt};
use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::{fs::File, io::AsyncWriteExt};

pub trait DatabaseWriter {
    fn write_account(&mut self, addr: Address, account: Account) -> Result<(), Error>;
    fn write_block(&mut self, block_hash: B256, block: SealedBlock) -> Result<(), Error>;
    fn write_transaction(&mut self, tx: Transaction) -> Result<(), Error>;
    fn write_transaction_receipt(
        &mut self,
        tx_hash: B256,
        tx_receipt: TransactionReceipt,
    ) -> Result<(), Error>;

    fn write_spec(&mut self, spec: &ChainSpec) -> Result<(), Error> {
        for (addr, account) in spec.iter_accounts() {
            self.write_account(*addr, *account)?;
        }

        Ok(())
    }
}

pub trait DatabaseReader {
    fn read_account(&self, addr: &Address) -> Option<&Account>;
    fn read_account_mut(&mut self, addr: &Address) -> Option<&mut Account>;
    fn read_transaction(&self, hash: &B256) -> Option<&Transaction>;
    fn read_block_by_hash(&self, block_hash: &B256) -> Option<&SealedBlock>;
    fn read_block_by_number(&self, block_number: u64) -> Option<&SealedBlock>;
    fn transaction_count(&self) -> usize;
    fn block_count(&self) -> usize;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InMemoryDB {
    accounts: HashMap<Address, Account>,
    blocks: HashMap<B256, SealedBlock>,
    /// Hashmap that stores each block hash by its number
    block_by_number: HashMap<u64, B256>,
    transactions: HashMap<B256, Transaction>,
    tx_receipts: HashMap<B256, TransactionReceipt>,
}

impl InMemoryDB {
    pub fn new() -> Self {
        Default::default()
    }
}

impl InMemoryDB {
    pub async fn mem_dump(&self, path: PathBuf) -> Result<(), Error> {
        let mut file = File::create(path).await.unwrap();
        file.write_all(serde_json::to_string_pretty(&self)?.as_bytes())
            .await?;
        Ok(())
    }
}

impl DatabaseWriter for InMemoryDB {
    fn write_account(&mut self, addr: Address, account: Account) -> Result<(), Error> {
        self.accounts.insert(addr, account);
        Ok(())
    }

    fn write_block(&mut self, block_hash: B256, block: SealedBlock) -> Result<(), Error> {
        for tx in block.transactions() {
            self.transactions.insert(tx.hash, tx.clone());
        }

        self.block_by_number.insert(block.number(), block_hash);
        self.blocks.insert(block_hash, block);

        Ok(())
    }

    fn write_transaction(&mut self, tx: Transaction) -> Result<(), Error> {
        self.transactions.insert(tx.hash, tx);
        Ok(())
    }

    fn write_transaction_receipt(
        &mut self,
        tx_hash: B256,
        tx_receipt: TransactionReceipt,
    ) -> Result<(), Error> {
        self.tx_receipts.insert(tx_hash, tx_receipt);
        Ok(())
    }
}

impl DatabaseReader for InMemoryDB {
    fn read_account(&self, addr: &Address) -> Option<&Account> {
        self.accounts.get(addr)
    }

    fn read_account_mut(&mut self, addr: &Address) -> Option<&mut Account> {
        self.accounts.get_mut(addr)
    }

    fn read_transaction(&self, hash: &B256) -> Option<&Transaction> {
        self.transactions.get(hash)
    }

    fn read_block_by_hash(&self, block_hash: &B256) -> Option<&SealedBlock> {
        self.blocks.get(block_hash)
    }

    fn read_block_by_number(&self, block_number: u64) -> Option<&SealedBlock> {
        let hash = self.block_by_number.get(&block_number)?;
        self.read_block_by_hash(hash)
    }

    fn block_count(&self) -> usize {
        self.blocks.len()
    }

    fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}
