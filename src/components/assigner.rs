use anyhow::Result;
use tracing::{info, error};
use subnet_wcp_persistence::{KvStore, keys};
use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::time::Duration;

const DEFAULT_TASK_KIND: &str = "video.preprocess";
const DEFAULT_TASK_VERSION: &str = "1.0.0";

#[derive(Serialize)]
struct InputDescriptor {
    name: String,
    media_type: String,
    #[serde(rename = "ref")]
    reference: String,
    inline_json: String,
    #[serde(with = "base64")]
    inline_bytes: Vec<u8>,
}

mod base64 {
    use serde::{Serialize, Serializer};
    use base64::{Engine as _, engine::general_purpose};
    
    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = general_purpose::STANDARD.encode(bytes);
        encoded.serialize(serializer)
    }
}

#[derive(Serialize)]
struct TaskAssignment {
    activity_id: String,
    workflow_instance_id: String,
    run_id: String,
    task_kind: String,
    task_version: String,
    inputs: Vec<InputDescriptor>,
    upload_prefix: String,
    soft_deadline_unix: i64,
    heartbeat_interval_s: i32,
}

#[derive(Deserialize)]
struct TaskResponse {
    message: String,
    task_id: String,
}

#[derive(Deserialize)]
struct TaskStatus {
    status: String,
    progress: u32,
    result_ref: Option<String>,
    error: Option<String>,
}

pub struct Assigner {
    store: KvStore,
    wep_endpoint: String,
    max_inflight: usize,
    client: Client,
}

impl Assigner {
    pub fn new(store: KvStore, wep_endpoint: String, max_inflight: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
            
        Self { 
            store, 
            wep_endpoint, 
            max_inflight,
            client,
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            let jobs = self.store.scan_prefix("inflight:")?;
            let mut in_progress = 0usize;
            for (k, _v) in jobs {
                if in_progress >= self.max_inflight { break; }
                let key = String::from_utf8_lossy(&k).to_string();
                if let Err(e) = self.handle_one_job(key).await {
                    error!(error=%e, "Error handling job");
                }
                in_progress += 1;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn handle_one_job(&self, key: String) -> Result<()> {
        info!(%key, "assigner handling job");
        
        // Dev short-circuit: allow bypassing WEP and marking completion
        if std::env::var("DEV_MOCK_ASSIGNER").is_ok() {
            let activity_id = key.replacen("inflight:", "", 1);
            info!(activity_id=%activity_id, "dev-mode: direct SUCCESS completion without WEP");
            let claim_key = keys::broadcast_complete(&activity_id);
            self.store.put(claim_key, b"1")?;
            let done_key = keys::done(&activity_id);
            self.store.put(done_key, b"ok")?;
            self.store.delete(keys::inflight(&activity_id).as_bytes())?;
            return Ok(());
        }

        let activity_id = key.replacen("inflight:", "", 1);
        let instance_id = "0xdeadbeef".to_string();
        
        // Create task assignment
        let assignment = TaskAssignment {
            activity_id: activity_id.clone(),
            workflow_instance_id: instance_id.clone(),
            run_id: "run-1".into(),
            task_kind: DEFAULT_TASK_KIND.into(),
            task_version: DEFAULT_TASK_VERSION.into(),
            inputs: vec![InputDescriptor { 
                name: "file_path".into(), 
                media_type: "text/plain".into(), 
                reference: "r2://bucket/path.mp4".into(), 
                inline_json: String::new(), 
                inline_bytes: Vec::new() 
            }],
            upload_prefix: "workflows/demo/preprocess".into(),
            soft_deadline_unix: 0,
            heartbeat_interval_s: 10,
        };
        
        // Send task assignment to WEP
        let assign_url = format!("{}/tasks/{}/assign", self.wep_endpoint, activity_id);
        let response = self.client
            .post(&assign_url)
            .json(&assignment)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                activity_id=%activity_id, 
                status=%status, 
                error=%error_text,
                "Failed to assign task to WEP"
            );
            return Ok(());
        }
        
        let _task_response: TaskResponse = response.json().await?;
        info!(activity_id=%activity_id, "Task assigned to WEP");
        
        // Update inflight status
        let inflight_key = keys::inflight(&activity_id);
        let inflight_val = format!(
            "{{\"activity_id\":\"{}\",\"instance_id\":\"{}\",\"wep_status\":\"Running\"}}", 
            activity_id, 
            instance_id
        );
        self.store.put(inflight_key.as_bytes(), inflight_val.as_bytes())?;
        
        // Poll for completion
        let status_url = format!("{}/tasks/{}/status", self.wep_endpoint, activity_id);
        let mut completed = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        
        while !completed && std::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            let response = self.client.get(&status_url).send().await?;
            if !response.status().is_success() {
                error!(activity_id=%activity_id, "Failed to get task status");
                break;
            }
            
            let status: TaskStatus = response.json().await?;
            info!(
                activity_id=%activity_id, 
                status=%status.status, 
                progress=%status.progress,
                "Task status update"
            );
            
            match status.status.as_str() {
                "completed" => {
                    // Task completed successfully
                    let claim_key = keys::broadcast_complete(&activity_id);
                    self.store.put(claim_key, b"1")?;
                    let resume_key = keys::broadcast_resume(&instance_id);
                    self.store.put(resume_key, b"1")?;
                    let done_key = keys::done(&activity_id);
                    self.store.put(done_key, b"ok")?;
                    self.store.delete(keys::inflight(&activity_id).as_bytes())?;
                    
                    info!(
                        activity_id=%activity_id,
                        result_ref=%status.result_ref.as_deref().unwrap_or(""),
                        "Task completed successfully"
                    );
                    completed = true;
                }
                "failed" => {
                    // Task failed
                    let done_key = keys::done(&activity_id);
                    self.store.put(done_key, b"failed")?;
                    self.store.delete(keys::inflight(&activity_id).as_bytes())?;
                    
                    error!(
                        activity_id=%activity_id,
                        error=%status.error.as_deref().unwrap_or(""),
                        "Task failed"
                    );
                    completed = true;
                }
                _ => {
                    // Still running, continue polling
                }
            }
        }
        
        if !completed {
            // Timeout - mark as failed
            error!(activity_id=%activity_id, "Task timed out");
            let done_key = keys::done(&activity_id);
            self.store.put(done_key, b"timeout")?;
            self.store.delete(keys::inflight(&activity_id).as_bytes())?;
        }
        
        Ok(())
    }
}
