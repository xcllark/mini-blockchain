use crate::{DatabaseReader, DatabaseWriter};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug)]
pub struct Reporter<DB> {
    db: Arc<RwLock<DB>>,
    frequency: u64,
}

impl<DB> Reporter<DB>
where
    DB: DatabaseWriter + DatabaseReader + Send + Sync + 'static,
{
    pub fn new(frequency: u64, db: Arc<RwLock<DB>>) -> Self {
        Self { frequency, db }
    }

    pub async fn run(self) {
        info!("Reporter Initialized Successfuly");
        loop {
            tokio::time::sleep(Duration::from_secs(self.frequency)).await;

            let db = self.db.read().await;

            info!(
                processed_blocks = db.block_count(),
                processed_transactions = db.transaction_count()
            );
        }
    }
}
