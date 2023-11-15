use alloy_primitives::Address;
use anyhow::Result;
use chain_bit::{ChainSpec, DatabaseWriter, Error, InMemoryDB, Reporter, Server};
use clap::{Args, Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::fs::File;
use std::{io::BufReader, path::PathBuf, sync::Arc};
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::info;

const BASE_PATH: &str = "~/.chain-bit";

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server(ServerArgs),
    Client,
}

#[derive(Args)]
struct ServerArgs {
    /// Path to the chainspec, if you want preallocations to
    /// your address specify it in the chainspec
    #[clap(long, short)]
    spec: Option<PathBuf>,

    /// Rpc Port
    #[clap(long, short, default_value = "8545")]
    port: u16,

    /// Coinbase address
    #[clap(
        long,
        short,
        default_value = "0x0000000000000000000000000000000000000000"
    )]
    coinbase: Address,

    /// Path where to dump the database at the end of execution
    #[clap(long)]
    database_dump: Option<PathBuf>,

    /// Wheter chain-bit should output debug info to the terminal
    /// For example when debug mode is activated every block will be
    /// printed to the terminal
    #[clap(short, long, default_value_t = false)]
    debug: bool,

    /// How often do you want info about the progress
    /// Let's you know how many blocks and transactions have
    /// been processesed
    #[clap(short, long, default_value_t = 30)]
    report_frequency: u64,

    /// Block time of the blockchain
    #[clap(short, long, default_value_t = 10)]
    block_time: u64,
}

fn read_file<T>(path: PathBuf) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.into())
}

pub fn create_folders(special_path: Option<PathBuf>) -> Result<()> {
    let path = if let Some(path) = special_path {
        path
    } else {
        PathBuf::from(BASE_PATH)
    };

    std::fs::create_dir_all(path)?;

    Ok(())
}

impl ServerArgs {
    pub fn set_tracing(&self) {
        let level = if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };

        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(level)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }

    pub async fn run(self) -> Result<()> {
        self.set_tracing();

        let spec: ChainSpec = if let Some(spec) = self.spec {
            read_file(spec)?
        } else {
            ChainSpec::default()
        };

        let mut database = InMemoryDB::default();
        database.write_spec(&spec)?;
        let database = Arc::new(RwLock::new(database));

        let reporter = Reporter::new(self.report_frequency, database.clone());
        tokio::spawn(reporter.run());

        let (notify_shutdown_tx, _) = broadcast::channel(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

        let server = Server::new(
            database.clone(),
            self.port,
            self.block_time,
            self.coinbase,
            notify_shutdown_tx,
            shutdown_complete_tx,
        );

        select! {
            _ = server.run() => {}
            _ = ctrl_c() => {
                info!("Ctrl-c received shutting down gracefully");
            }
        }

        let Server {
            notify_shutdown,
            shutdown_complete_tx,
            ..
        } = server;

        drop(shutdown_complete_tx);
        drop(notify_shutdown);

        if let Some(ref path) = self.database_dump {
            info!("Dumping database");
            let path = path.join("database.json");
            let db = database.read().await;
            db.mem_dump(path).await?;
        }

        info!("Waiting for other tasks to complete");
        let _ = shutdown_complete_rx.recv().await;
        info!("Shutdown complete");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server(server) => {
            server.run().await?;
        }

        Commands::Client => {
            chain_bit::client::run_loop().await?;
        }
    }

    Ok(())
}
