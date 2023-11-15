use crate::{
    database::{DatabaseReader, DatabaseWriter},
    error::Error,
    server::connection::Connection,
    Transaction,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::error;

use super::{
    message::{BlockReq, TransactionReq},
    Message,
};

pub struct Handler<DB> {
    /// Shared InMemoryDB handle
    db: Arc<RwLock<DB>>,

    /// TcpConnection wrapper
    connection: Connection,

    /// Sender half of [mpsc] channel, that allows to send [Transaction]
    /// to the mempool from each handler
    server_mempool_tx: mpsc::Sender<Transaction>,
}

impl<DB> Handler<DB>
where
    DB: DatabaseReader + DatabaseWriter + Send + Sync + 'static,
{
    pub fn new(
        db: Arc<RwLock<DB>>,
        connection: Connection,
        server_mempool_tx: mpsc::Sender<Transaction>,
    ) -> Self {
        Self {
            db,
            connection,
            server_mempool_tx,
        }
    }

    pub async fn shutdown(self) {
        self.connection.shutdown().await;
    }

    pub async fn handle_connection(mut self) {
        let msg = match self.connection.read_message().await {
            Ok(msg) => msg,
            Err(e) => {
                error!(err = %e, "Couldn't read message from connection, closing connection");
                self.shutdown().await;
                return;
            }
        };

        let msg = match msg {
            Some(msg) => msg,
            None => {
                error!("No message received from the connection, closing connection");
                self.shutdown().await;
                return;
            }
        };

        let response = match self.handle_message(msg).await {
            Ok(resp) => resp,
            Err(e) => {
                error!(err = %e, "Couldn't handle message, closing connection");
                self.shutdown().await;
                return;
            }
        };

        match self.connection.write_message(&response).await {
            Ok(_) => (),
            Err(e) => {
                error!(err = %e, "Couldn't handle message, closing connection");
            }
        }

        self.shutdown().await;
    }

    pub async fn handle_message(&mut self, msg: Message) -> Result<Message, Error> {
        match msg {
            Message::Transaction(tx) => self.handle_transaction(tx).await,
            Message::BlockReq(req) => self.handle_block_req(req).await,
            Message::TransactionReq(req) => self.handle_transaction_req(req).await,

            Message::Block(_) | Message::Blocks(_) => Ok(Message::InvalidMessage(String::from(
                "The rpc server doesn't expect blocks",
            ))),

            Message::InvalidMessage(_)
            | Message::Ok
            | Message::InternalError(_)
            | Message::InvalidTransaction
            | Message::NonExistentBlock
            | Message::NonExistentTx => Ok(Message::InvalidMessage(String::new())),
        }
    }

    pub async fn handle_transaction(&self, tx: Transaction) -> Result<Message, Error> {
        let (result, tx) = tokio::task::spawn_blocking(move || (tx.verify(), tx)).await?;

        if !result {
            return Ok(Message::InvalidTransaction);
        }

        // Send the transaction to the mempool to include it into the mempool
        if let Err(e) = self.server_mempool_tx.send(tx).await {
            error!(err = %e, "Couldn't send transaction over the channel to the mempool");
            return Ok(Message::InternalError(format!("Internal error: {}", e)));
        }

        Ok(Message::Ok)
    }

    pub async fn handle_block_req(&self, block_req: BlockReq) -> Result<Message, Error> {
        let db = self.db.read().await;
        let block = match block_req {
            BlockReq::Hash(hash) => db.read_block_by_hash(&hash),
            BlockReq::Number(number) => db.read_block_by_number(number),
            BlockReq::Range { .. } => unimplemented!("Block range is not yet implemented"),
        };

        match block {
            Some(block) => Ok(Message::Block(block.clone())),
            None => Ok(Message::NonExistentBlock),
        }
    }

    pub async fn handle_transaction_req(&self, tx_req: TransactionReq) -> Result<Message, Error> {
        let db = self.db.read().await;

        let transaction = match tx_req {
            TransactionReq::Hash(hash) => db.read_transaction(&hash),
            TransactionReq::Many(_) => {
                unimplemented!("Can't yet ask for many transactions at once")
            }
        };

        match transaction {
            Some(tx) => Ok(Message::Transaction(tx.clone())),
            None => Ok(Message::NonExistentTx),
        }
    }
}
