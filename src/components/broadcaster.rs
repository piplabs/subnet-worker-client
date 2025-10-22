use anyhow::Result;
use tracing::{info, warn};
use subnet_wcp_persistence::KvStore;
use subnet_wcp_persistence::keys;
use alloy::providers::Provider;
use alloy::primitives::Address;

#[derive(Clone)]
pub struct Broadcaster<P: Provider + Clone + Send + Sync + 'static> {
    store: KvStore,
    provider: P,
    task_queue: Address,
    workflow_engine: Address,
}

impl<P: Provider + Clone + Send + Sync + 'static> Broadcaster<P> {
    pub fn new(store: KvStore, provider: P, task_queue: Address, workflow_engine: Address) -> Self {
        Self { store, provider, task_queue, workflow_engine }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            // Claim txs
            for (k, _v) in self.store.scan_prefix("broadcast:claim:")? {
                let key = String::from_utf8_lossy(&k).to_string();
                if let Some(id) = key.strip_prefix("broadcast:claim:") {
                    self.send_claim(id).await?;
                }
            }
            // Complete txs
            for (k, _v) in self.store.scan_prefix("broadcast:complete:")? {
                let key = String::from_utf8_lossy(&k).to_string();
                if let Some(id) = key.strip_prefix("broadcast:complete:") {
                    self.send_complete(id).await?;
                }
            }
            // Resume txs
            for (k, _v) in self.store.scan_prefix("broadcast:resume:")? {
                let key = String::from_utf8_lossy(&k).to_string();
                if let Some(instance) = key.strip_prefix("broadcast:resume:") {
                    self.send_resume(instance).await?;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    async fn send_claim(&self, activity_id: &str) -> Result<()> {
        info!(%activity_id, "broadcast claim (stub)");
        // TODO: build and submit claimActivity tx via provider + wallet
        self.store.delete(keys::broadcast_claim(activity_id).as_bytes())?;
        Ok(())
    }

    async fn send_complete(&self, activity_id: &str) -> Result<()> {
        info!(%activity_id, "broadcast complete (stub)");
        // TODO: build and submit completeActivity tx via provider + wallet
        self.store.delete(keys::broadcast_complete(activity_id).as_bytes())?;
        Ok(())
    }

    async fn send_resume(&self, instance_id: &str) -> Result<()> {
        info!(%instance_id, "broadcast resume (stub)");
        // TODO: build and submit resumeWorkflow tx via provider + wallet
        self.store.delete(keys::broadcast_resume(instance_id).as_bytes())?;
        Ok(())
    }
}


