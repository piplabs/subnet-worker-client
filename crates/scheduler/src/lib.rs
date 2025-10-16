use anyhow::Result;
use subnet_wcp_persistence::{KvStore, keys};
use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use alloy::providers::Provider;
use alloy::primitives::{Address, B256};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimJob {
    pub activity_id: String,
    pub queue_name: String,
    pub created_at_ms: u128,
}

pub struct Scheduler<P: Provider + Clone + Send + Sync + 'static> {
    store: KvStore,
    poll_interval: Duration,
    queue_name: String,
    provider: P,
    task_queue_addr: Address,
}

impl<P: Provider + Clone + Send + Sync + 'static> Scheduler<P> {
    pub fn new(store: KvStore, poll_interval: Duration, queue_name: String, provider: P, task_queue_addr: Address) -> Self {
        Self { store, poll_interval, queue_name, provider, task_queue_addr }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            if let Some(id) = poll_once(&self.provider, self.task_queue_addr, &self.queue_name).await? {
                let job = ClaimJob {
                    activity_id: format!("0x{:x}", id),
                    queue_name: self.queue_name.clone(),
                    created_at_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                };
                let key = keys::claim_job(&job.activity_id);
                let val = serde_json::to_vec(&job)?;
                self.store.put(key, val)?;
                info!(activity_id = %job.activity_id, "enqueued claim job");
            }
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

async fn poll_once<P: Provider + Clone + Send + Sync + 'static>(provider: &P, task_queue_addr: Address, queue: &str) -> Result<Option<B256>> {
    crate::chain::task_queue::poll_activity(provider, task_queue_addr, queue).await
}


