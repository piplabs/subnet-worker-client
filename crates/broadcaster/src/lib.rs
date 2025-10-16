use anyhow::Result;
use subnet_wcp_persistence::{KvStore, keys};
use tracing::{info, warn};
use subnet_wcp_rpc::{execution_v1::{self, Envelope, TaskAssignment, InputDescriptor, Version, Capabilities}, WepGrpcClient};
use tokio_stream::StreamExt;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct ActivitySpec {
    task_kind: String,
    task_version: String,
    inputs: Vec<SpecField>,
}
#[derive(Deserialize)]
struct SpecField { name: String, r#type: String }

pub struct Broadcaster<P> {
    store: KvStore,
    provider: P,
}

impl<P> Broadcaster<P> {
    pub fn new(store: KvStore, provider: P) -> Self { Self { store, provider } }
}

impl<P> Broadcaster<P> {
    pub async fn run_claim_loop(&self) -> Result<()> {
        loop {
            let jobs = self.store.scan_prefix("claim_job:")?;
            if jobs.is_empty() {
                // Seed a demo job for MVP if none present
                let demo_key = "claim_job:0xabc";
                let _ = self.store.put(demo_key.as_bytes(), b"{}");
            }
            for (k, v) in jobs {
                let key = String::from_utf8_lossy(&k).to_string();
                info!(%key, "found claim job");
                // Load spec (MVP hardcoded path)
                let spec_str = fs::read_to_string("activity-specs/video.preprocess.yaml")?;
                let spec: ActivitySpec = serde_yaml::from_str(&spec_str)?;

                // Open gRPC stream to WEP (MVP local endpoint)
                let mut client = WepGrpcClient::connect("http://127.0.0.1:7070").await?;
                let (mut tx, mut rx) = client.open_task_stream().await?;

                // Send hello + capabilities
                let _ = tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Hello(Version{ min: "1.0.0".into(), max: "1.0.0".into() })) }).await;
                let _ = tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Capabilities(Capabilities{ max_concurrency: 4, tags: vec!["cpu".into()] })) }).await;

                // Build assignment (MVP with a fake object_key)
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
                let _ = tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Assign(assignment)) }).await;

                // Await completion (MVP)
                while let Some(Ok(env)) = rx.next().await {
                    if let Some(execution_v1::envelope::Msg::Completion(c)) = env.msg { 
                        info!(activity_id = %c.activity_id, status = %c.status, "received completion");
                        // Mark done and delete job
                        let done_key = keys::done(&c.activity_id);
                        self.store.put(done_key, b"ok")?;
                        self.store.delete(key.as_bytes())?;
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

