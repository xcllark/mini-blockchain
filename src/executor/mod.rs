mod mempool;

use crate::{
    database::{DatabaseReader, DatabaseWriter},
    Account, Block, BlockHeader, ChangeSet, Error, SealedBlock, Shutdown, State,
    TransactionReceipt, Transactions,
};
use alloy_primitives::{Address, B256, U256};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
};
use tracing::{debug, error, info};
const INITIAL_HASH: B256 = B256::ZERO;

pub use mempool::{Mempool, MempoolOrdering};
pub type ExecutorMempoolTx = UnboundedSender<oneshot::Sender<Transactions>>;
pub type ExecutorMempoolRx = UnboundedReceiver<oneshot::Sender<Transactions>>;

#[derive(Debug)]
pub struct Executor<DB> {
    pub db: Arc<RwLock<DB>>,
    pub executor_mempool_tx: ExecutorMempoolTx,
    pub block_time: u64,
    pub coinbase: Address,
    pub last_hash: B256,
    pub next_number: u64,
    pub shutdown: Shutdown,
    pub _shutdown_complete: mpsc::Sender<()>,
}

impl<DB> Executor<DB>
where
    DB: DatabaseWriter + DatabaseReader + Send + Sync + 'static,
{
    pub fn new(
        db: Arc<RwLock<DB>>,
        block_time: u64,
        executor_mempool_tx: ExecutorMempoolTx,
        coinbase: Address,
        shutdown: broadcast::Receiver<()>,
        shutdown_complete: mpsc::Sender<()>,
    ) -> Self {
        Self {
            executor_mempool_tx,
            coinbase,
            block_time,
            db,
            last_hash: INITIAL_HASH,
            next_number: 1,
            shutdown: Shutdown::new(shutdown),
            _shutdown_complete: shutdown_complete,
        }
    }
    pub async fn run(mut self) -> Result<(), Error> {
        info!("Executor Initialized Successfuly");
        let mut interval = tokio::time::interval(Duration::from_secs(self.block_time));
        interval.tick().await;

        while !self.shutdown.is_shutdown() {
            select! {
                _ = interval.tick() => {}
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            }

            let block = match self.build_block().await {
                Ok(block) => block,
                Err(e) => {
                    error!(err = %e, "Failed to get transactions from mempool, retrying...");
                    continue;
                }
            };

            debug!("\n{:#?}", block);

            let block_hash = *block.get_hash();

            // Get read lock since for executing the transactions we only need to read the db
            let db = self.db.read().await;

            let change_set = self.execute_transactions(&db, &block).into();

            // Here we have to drop the db_reader otherwise we just shadow it in the next line
            // And create a deadlock, because this lock will be dropped at the end of the scope
            // even if shadowed, thus the next line will wait forever
            drop(db);

            let mut db = self.db.write().await;

            if let Err(e) = self.write_changeset(&mut db, change_set) {
                error!(err = %e, "Couldn't write change_set to database, skipping");
                continue;
            }

            if let Err(e) = self.write_block(&mut db, block) {
                error!(err = %e, "Couldn't write change_set to database, skipping");
                continue;
            }

            // We always want to drop the lock as soon as possible
            drop(db);

            self.last_hash = block_hash;
            self.next_number += 1;
        }
        Ok(())
    }

    pub async fn build_block(&self) -> Result<SealedBlock, Error> {
        let (oneshot_tx, oneshot_rx) = oneshot::channel();
        self.executor_mempool_tx
            .send(oneshot_tx)
            .map_err(|_| Error::ChannelFailure)?;

        let transactions = oneshot_rx.await.map_err(|_| Error::ChannelFailure)?;

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let tx_root = transactions.get_root();

        let header = BlockHeader {
            parent_hash: self.last_hash,
            nonce: 0,
            difficulty: U256::MAX,
            number: self.next_number,
            timestamp,
            coinbase: self.coinbase,
            tx_root,
        };

        let block = Block::new(header, transactions);
        Ok(block.seal_slow())
    }

    /// Executes all transactions in a given block and produces [ChangeSet]
    /// This changeset will be later written to the database
    ///
    /// Notice that we are using the [RwLockReadGuard] not the write guard.
    /// This is to not block other parts of the server, so they can still
    /// respond to requests to read the db
    ///
    /// But beacause of this we don't update the database after every transaction,
    /// Instead we build a [ChangeSet] and get the latest state from there.
    pub fn execute_transactions<'a>(
        &self,
        db: &'a RwLockReadGuard<'a, DB>,
        block: &SealedBlock,
    ) -> State<'a, DB> {
        let mut state = State::new(db);

        for tx in block.transactions() {
            let tx_hash = tx.get_hash();
            let mut receipt = TransactionReceipt::build(tx, block);

            let mut from_account = match state.get_account(&tx.from) {
                Some(account) => *account,
                None => {
                    state.insert_receipt(&tx_hash, receipt);
                    continue;
                }
            };

            if from_account.nonce() != tx.nonce {
                state.insert_receipt(&tx_hash, receipt);
                continue;
            }

            // We first check the changeset to make sure we have the latest state
            let mut to_account = match state.get_account(&tx.to) {
                Some(account) => *account,
                None => Account::default(),
            };

            if from_account.balance() < tx.value {
                state.insert_receipt(&tx_hash, receipt);
                continue;
            }

            let new_from_balance = from_account.balance() - tx.value;
            from_account.update_balance(new_from_balance);

            let new_to_balance = to_account.balance() + tx.value;
            to_account.update_balance(new_to_balance);

            receipt.success = true;

            from_account.increment_nonce();

            state.insert_receipt(&tx_hash, receipt);
            state.insert_account(&tx.from, from_account);
            state.insert_account(&tx.to, to_account);
        }

        state
    }

    /// Writes the block to the database
    pub fn write_block(
        &self,
        db: &mut RwLockWriteGuard<'_, DB>,
        block: SealedBlock,
    ) -> Result<(), Error> {
        let block_hash = *block.get_hash();
        db.write_block(block_hash, block)?;

        Ok(())
    }

    pub fn write_changeset(
        &self,
        db: &mut RwLockWriteGuard<'_, DB>,
        changeset: ChangeSet,
    ) -> Result<(), Error> {
        // TODO: Remove the two loops and introduce some better mechanism
        // to write the receipts and touched accounts
        for (addr, account) in changeset.touched_accounts {
            if let Err(e) = db.write_account(addr, account) {
                error!(err = %e, "Couldn't write account to database, skipping...");
            }
        }

        for (tx_hash, tx_receipt) in changeset.receipts {
            if let Err(e) = db.write_transaction_receipt(tx_hash, tx_receipt) {
                error!(err = %e, "Couldn't write transaction receipt to database, skipping...");
            }
        }

        Ok(())
    }
}
