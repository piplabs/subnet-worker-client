use anyhow::Result;
use alloy::providers::Provider;
use alloy::primitives::Address;

pub mod control_plane {
    use super::*;
    pub async fn is_worker_active<P: Provider + Clone + Send + Sync + 'static>(_: &P, _: Address, _: Address) -> Result<bool> {
        Ok(true)
    }
    pub async fn get_protocol_version<P: Provider + Clone + Send + Sync + 'static>(_: &P, _: Address) -> Result<String> {
        Ok("0.2.0".to_string())
    }
}

pub mod task_queue {
    use super::*;
    use anyhow::Result;
    pub async fn poll_activity<P: Provider + Clone + Send + Sync + 'static>(_: &P, _: Address, _: &str) -> Result<Option<[u8;32]>> {
        Ok(None)
    }
}


