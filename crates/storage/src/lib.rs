//! Subnet API storage broker skeleton
use anyhow::Result;

pub struct StorageBroker {
    pub endpoint: String,
}

impl StorageBroker {
    pub fn new(endpoint: String) -> Self { Self { endpoint } }

    pub async fn get_presigned_download(&self, _key: &str) -> Result<String> {
        // TODO: gRPC call to subnet-api
        Ok("https://example.com/presigned".to_string())
    }
}


