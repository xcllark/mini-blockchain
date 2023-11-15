use std::collections::VecDeque;

use crate::{Error, Shutdown, Transaction, Transactions};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use tracing::info;
use super::ExecutorMempoolRx;

#[derive(Debug, Default)]
pub enum MempoolOrdering {
    #[default]
    Fifo,
}

#[derive(Debug)]
pub struct Mempool {
    /// We use [VecDeque] so we can pop from the front and use the [MempoolOrdering::Fifo] ordering
    transactions: VecDeque<Transaction>,

    server_mempool_rx: mpsc::Receiver<Transaction>,
    executor_mempool_rx: ExecutorMempoolRx,

    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,

    ordering: MempoolOrdering,
}

impl Mempool {
    pub fn new(
        server_mempool_rx: mpsc::Receiver<Transaction>,
        executor_mempool_rx: ExecutorMempoolRx,
        ordering: MempoolOrdering,
        shutdown: broadcast::Receiver<()>,
        _shutdown_complete: mpsc::Sender<()>,
    ) -> Self {
        Self {
            transactions: VecDeque::new(),
            server_mempool_rx,
            executor_mempool_rx,
            ordering,
            shutdown: Shutdown::new(shutdown),
            _shutdown_complete,
        }
    }

    pub async fn run(mut self) -> Result<(), Error> {

        info!("Mempool Initialized Successfuly");

        while !self.shutdown.is_shutdown() {
            select! {
                // Sender part of this channel is cloned to every single connection
                tx = self.server_mempool_rx.recv() => {
                    let tx = tx.ok_or(Error::ChannelFailure)?;
                    self.push(tx);
                },

                oneshot = self.executor_mempool_rx.recv() => {
                    let oneshot = oneshot.ok_or(Error::ChannelFailure)?;
                    let transactions = self.get_transactions();
                    oneshot.send(transactions).map_err(|_| Error::ChannelFailure)?;
                }
            }
        }

        Ok(())
    }

    pub fn push(&mut self, tx: Transaction) {
        self.transactions.push_back(tx);
    }

    pub fn pop(&mut self) -> Option<Transaction> {
        match self.ordering {
            MempoolOrdering::Fifo => self.transactions.pop_front(),
        }
    }

    pub fn get_transactions(&mut self) -> Transactions {
        let mut transactions = Vec::new();
        // TODO: Make this more efficient with mem::swap or mem::copy or somthing
        while let Some(tx) = self.pop() {
            transactions.push(tx);
            if transactions.len() >= 100 {
                break;
            }
        }

        let mut transactions: Transactions = transactions.into();
        transactions.sort();
        transactions
    }
}
