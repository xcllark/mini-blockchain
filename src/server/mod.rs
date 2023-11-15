mod connection;
mod handler;
mod message;
mod black_list;

pub use connection::Connection;
pub use message::Message;
use crate::executor::MempoolOrdering;

use crate::{
    database::{DatabaseReader, DatabaseWriter},
    executor::Mempool,
    server::{handler::Handler, black_list::BlackList},
    Error, Executor,
};
use alloy_primitives::Address;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{
        broadcast,
        mpsc::{self, unbounded_channel},
        RwLock,
    },
};
use tracing::{error, info};

pub struct Server<DB> {
    /// Port on where the server will listen
    port: u16,
    /// Arc copy to the database, database can be any data structure that implementes 
    /// [DatabaseReader] and [DatabaseWriter]
    db: Arc<RwLock<DB>>,

    /// Coinbase address of the executor
    ///
    /// This is technically not nesessary since we are not giving any rewards for mining a new
    /// block
    coinbase: Address,

    /// Here we specify how often we want the [Executor] to create a new block
    /// 
    /// Ethereum: 12 Seconds
    /// Bitcoin: 10 Minutes
    block_time: u64,

    /// These two channels are here to shutdown gracefully when the user presses ctrl-c in his
    /// termial
    ///
    /// [broadcast::Receiver] is sent to every task, and when we want to shut them down, we drop
    /// this [broadcast::Sender] which will send a signal to the [broadcast::Receiver] that he
    /// should shutdown
    ///
    /// Implementation of this is in the [crate::Shutdown] struct
    pub notify_shutdown: broadcast::Sender<()>,

    /// Every task holds the [mpsc::Sender] as well. Once we send the shutdown signal we wait one
    /// [mpsc::Receiver] which will receive once all Senders are dropped thus ensuring all tasks
    /// were sutdown successfuly
    pub shutdown_complete_tx: mpsc::Sender<()>,
}

impl<DB> Server<DB>
where
    DB: DatabaseReader + DatabaseWriter + Send + Sync + 'static,
{
    /// Creates a new Server
    pub fn new(
        db: Arc<RwLock<DB>>,
        port: u16,
        block_time: u64,
        coinbase: Address,
        notify_shutdown: broadcast::Sender<()>,
        shutdown_complete_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            port,
            db,
            block_time,
            coinbase,
            notify_shutdown,
            shutdown_complete_tx,
        }
    }

    /// Runs the server
    pub async fn run(&self) -> Result<(), Error> {
        let (server_mempool_tx, server_mempool_rx) = mpsc::channel(1000);
        let (executor_mempool_tx, executor_mempool_rx) = unbounded_channel();


        let /*mut*/ black_list = BlackList::default();

        let executor = Executor::new(
            self.db.clone(),
            self.block_time,
            executor_mempool_tx,
            self.coinbase,
            self.notify_shutdown.subscribe(),
            self.shutdown_complete_tx.clone(),
        );

        let mempool = Mempool::new(
            server_mempool_rx,
            executor_mempool_rx,
            MempoolOrdering::Fifo,
            self.notify_shutdown.subscribe(),
            self.shutdown_complete_tx.clone(),
        );

        tokio::spawn(mempool.run());
        tokio::spawn(executor.run());

        let server = TcpListener::bind(format!("localhost:{}", self.port)).await?;
        info!("Rpc Server Initialized Successfuly");

        loop {
            let (stream, ip_addr) = match server.accept().await {
                Ok(info) => info,
                Err(e) => {
                    error!(err = %e, "Couldn't accept connection, skipping");
                    continue;
                }
            };

            if black_list.contains(&ip_addr) {
                // In case the ip is on the blacklist we skipt the connection
                continue;
            }

            let connection = Connection::new(stream);
            let handler = Handler::new(self.db.clone(), connection, server_mempool_tx.clone());

            tokio::spawn(handler.handle_connection());
        }
    }
}
