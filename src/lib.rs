mod chainspec;
pub mod client;
mod database;
mod error;
mod executor;
mod primitives;
mod report;
mod server;
pub mod utils;

pub use chainspec::ChainSpec;
pub use database::{DatabaseReader, DatabaseWriter, InMemoryDB};
pub use error::Error;
pub use executor::Executor;
pub use primitives::*;
pub use report::Reporter;
pub use server::Server;
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Shutdown {
    pub(crate) is_shutdown: bool,
    pub(crate) notify: broadcast::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            is_shutdown: false,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.is_shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.is_shutdown = true;
    }
}
