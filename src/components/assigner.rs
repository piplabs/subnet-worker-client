use anyhow::Result;
use tracing::info;
use tokio_stream::StreamExt;
use subnet_wcp_persistence::{KvStore, keys};
use subnet_wcp_rpc::{execution_v1::{self, Envelope, TaskAssignment, InputDescriptor, Version, Capabilities}, WepGrpcClient};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct ActivitySpec {
    task_kind: String,
    task_version: String,
}

pub struct Assigner {
    store: KvStore,
    wep_endpoint: String,
    max_inflight: usize,
}

impl Assigner {
    pub fn new(store: KvStore, wep_endpoint: String, max_inflight: usize) -> Self {
        Self { store, wep_endpoint, max_inflight }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            let jobs = self.store.scan_prefix("claim_job:")?;
            let mut in_progress = 0usize;
            for (k, _v) in jobs {
                if in_progress >= self.max_inflight { break; }
                let key = String::from_utf8_lossy(&k).to_string();
                self.handle_one_job(key).await?;
                in_progress += 1;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    async fn handle_one_job(&self, key: String) -> Result<()> {
        info!(%key, "assigner handling job");
        let spec_str = fs::read_to_string("activity-specs/video.preprocess.yaml")?;
        let spec: ActivitySpec = serde_yaml::from_str(&spec_str)?;

        let mut client = WepGrpcClient::connect(&self.wep_endpoint).await?;
        let (tx, mut rx) = client.open_task_stream().await?;

        // Handshake
        let _ = tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Hello(Version{ min: "1.0.0".into(), max: "1.0.0".into() })) }).await;
        if let Some(Ok(env)) = rx.next().await {
            if let Some(execution_v1::envelope::Msg::HelloAck(v)) = env.msg { info!(min=%v.min, max=%v.max, "wep hello_ack"); }
        }
        let _ = tx.send(Envelope { msg: Some(execution_v1::envelope::Msg::Capabilities(Capabilities{ max_concurrency: self.max_inflight as u32, tags: vec!["cpu".into()] })) }).await;

        // Assign task
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

        while let Some(Ok(env)) = rx.next().await {
            if let Some(execution_v1::envelope::Msg::Completion(c)) = env.msg {
                info!(activity_id = %c.activity_id, status = %c.status, "assigner received completion");
                let done_key = keys::done(&c.activity_id);
                self.store.put(done_key, b"ok")?;
                self.store.delete(key.as_bytes())?;
                break;
            }
        }
        Ok(())
    }
}


