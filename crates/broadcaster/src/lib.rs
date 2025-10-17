use anyhow::Result;
use subnet_wcp_persistence::{KvStore, keys};
use tracing::{info, warn};
use subnet_wcp_rpc::{execution_v1::{self, Envelope, TaskAssignment, InputDescriptor, Version, Capabilities}, WepGrpcClient};
use tokio_stream::StreamExt;
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use tokio::task::JoinSet;

#[derive(Deserialize)]
struct ActivitySpec {
    task_kind: String,
    task_version: String,
    inputs: Vec<SpecField>,
}
#[derive(Deserialize)]
struct SpecField { name: String, r#type: String }

#[derive(Clone)]
pub struct BroadcasterConfig {
    pub wep_endpoint_http: String,
    pub max_inflight: usize,
}

pub struct Broadcaster<P> {
    store: KvStore,
    provider: P,
    cfg: Arc<BroadcasterConfig>,
}

impl<P> Broadcaster<P> {
    pub fn new_with_cfg(store: KvStore, provider: P, cfg: BroadcasterConfig) -> Self {
        Self { store, provider, cfg: Arc::new(cfg) }
    }
    pub fn new(store: KvStore, provider: P) -> Self { // default
        Self::new_with_cfg(store, provider, BroadcasterConfig { wep_endpoint_http: "http://127.0.0.1:7070".into(), max_inflight: 4 })
    }
}

impl<P> Broadcaster<P> {
    pub async fn run_claim_loop(&self) -> Result<()> {
        loop {
            let jobs = self.store.scan_prefix("claim_job:")?;
            if jobs.is_empty() {
                for i in 0..self.cfg.max_inflight.max(4) {
                    let demo_key = format!("claim_job:0xabc{:x}", i);
                    let _ = self.store.put(demo_key.as_bytes(), b"{}");
                }
            }

            let mut joinset = JoinSet::new();
            for (k, _v) in jobs {
                while joinset.len() >= self.cfg.max_inflight { let _ = joinset.join_next().await; }
                let key = String::from_utf8_lossy(&k).to_string();
                let store = self.store.clone();
                let wep_endpoint = self.cfg.wep_endpoint_http.clone();
                joinset.spawn(async move {
                    if let Err(e) = handle_one_job(store, key, wep_endpoint).await {
                        warn!(error = %e, "job handling failed");
                    }
                });
            }
            while joinset.len() > 0 { let _ = joinset.join_next().await; }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

async fn handle_one_job(store: KvStore, key: String, wep_endpoint: String) -> Result<()> {
    info!(%key, "handling claim job");
    let spec_str = fs::read_to_string("activity-specs/video.preprocess.yaml")?;
    let spec: ActivitySpec = serde_yaml::from_str(&spec_str)?;

    let mut client = WepGrpcClient::connect(wep_endpoint).await?;
    let (mut tx, mut rx) = client.open_task_stream().await?;

    match tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Hello(Version{ min: "1.0.0".into(), max: "1.0.0".into() })) }).await {
        Ok(_) => info!(%key, "sent hello"),
        Err(e) => warn!(%key, error=%e, "failed to send hello"),
    }
    match tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Capabilities(Capabilities{ max_concurrency: 4, tags: vec!["cpu".into()] })) }).await {
        Ok(_) => info!(%key, "sent capabilities"),
        Err(e) => warn!(%key, error=%e, "failed to send capabilities"),
    }

    let assignment = TaskAssignment{
        activity_id: key.replacen("claim_job:", "", 1),
        workflow_instance_id: "0xdeadbeef".into(),
        run_id: "run-1".into(),
        task_kind: spec.task_kind,
        task_version: spec.task_version,
        inputs: vec![InputDescriptor{ name: "object_key".into(), media_type: "text/plain".into(), r#ref: "r2://bucket/path.mp4".into(), inline_json: String::new(), inline_bytes: Vec::new() }],
        upload_prefix: "workflows/demo/preprocess".into(),
        soft_deadline_unix: 0,
        heartbeat_interval_s: 10,
    };
    match tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Assign(assignment)) }).await {
        Ok(_) => info!(%key, "sent assignment"),
        Err(e) => warn!(%key, error=%e, "failed to send assignment"),
    }

    while let Some(Ok(env)) = rx.next().await {
        if let Some(execution_v1::envelope::Msg::Completion(c)) = env.msg {
            info!(activity_id = %c.activity_id, status = %c.status, "received completion");
            let done_key = keys::done(&c.activity_id);
            store.put(done_key, b"ok")?;
            store.delete(key.as_bytes())?;
            break;
        }
    }
    Ok(())
}

