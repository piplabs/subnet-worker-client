use anyhow::Result;
use tracing::info;
use subnet_wcp_persistence::KvStore;
use subnet_wcp_persistence::keys;
use alloy::providers::Provider;
use alloy::primitives::{Address, B256, hex};
use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
use chrono::Utc;

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
        // Simplified dev path: just drain claim intents → inflight
        self.submit_claims_loop().await
    }

    async fn send_claim(&self, activity_id: &str) -> Result<()> {
        sol! {
            #[sol(rpc)]
            contract TaskQueue {
                function claimActivity(bytes32 activityId) external returns (bool);
            }
        }
        // Parse 0x-prefixed bytes32 id
        let id = activity_id.trim_start_matches("0x");
        let bytes = hex::decode(id)?;
        let _activity_b256: B256 = B256::from_slice(&bytes);
        let _tq = TaskQueue::new(self.task_queue, self.provider.clone());

        // Dev: record placeholder tx and assume instant confirmation
        let tx_rec = serde_json::json!({
            "activity_id": activity_id,
            "kind": "claim",
            "tx_hash": "0xdevmock",
            "submitted_at_ms": Utc::now().timestamp_millis(),
        });
        self.store.put(keys::tx(activity_id), serde_json::to_vec(&tx_rec)?)?;
        info!(%activity_id, "claim submitted (dev-mock)");

        // On confirm: move to inflight and remove claim job/tx entry as needed
        let inflight = serde_json::json!({
            "activity_id": activity_id,
            "queue": "", // unknown here; poller/assigner can enrich
            "claimed_at_ms": Utc::now().timestamp_millis(),
            "assignment_status": "Pending",
        });
        self.store.put(keys::inflight(activity_id), serde_json::to_vec(&inflight)?)?;
        self.store.delete(keys::broadcast_claim(activity_id).as_bytes())?;
        // keep tx record for audit; optionally remove/comment below
        // self.store.delete(keys::tx(activity_id).as_bytes())?;

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

    async fn submit_claims_loop(&self) -> Result<()> {
        loop {
            for (k, _v) in self.store.scan_prefix("broadcast:claim:")? {
                let key = String::from_utf8_lossy(&k).to_string();
                if let Some(activity_id) = key.strip_prefix("broadcast:claim:") {
                    // Skip if tx already submitted for this activity
                    if let Some(existing) = self.load_tx(activity_id)? {
                        if existing.status == "submitted" || existing.status == "confirmed" {
                            continue;
                        }
                    }
                    let _ = self.send_claim(activity_id).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    async fn confirm_txs_loop(&self) -> Result<()> { Ok(()) }

    async fn bump_txs_loop(&self) -> Result<()> { Ok(()) }

    fn load_tx(&self, activity_id: &str) -> Result<Option<TxRecord>> {
        if let Some(v) = self.store.get(keys::tx(activity_id))? {
            let rec: TxRecord = serde_json::from_slice(&v)?;
            Ok(Some(rec))
        } else {
            Ok(None)
        }
    }

    fn save_tx(&self, rec: &TxRecord) -> Result<()> {
        self.store.put(keys::tx(&rec.activity_id), serde_json::to_vec(rec)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxRecord {
    activity_id: String,
    kind: String,
    status: String, // pending|submitted|confirmed|dropped|replaced
    tx_hash: Option<String>,
    submitted_at_ms: i64,
    last_bump_at_ms: Option<i64>,
}


