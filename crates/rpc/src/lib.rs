//! WEP RPC client skeleton
use anyhow::Result;
use tonic::transport::Channel;

pub struct WepClient {
    #[allow(dead_code)]
    channel: Channel,
}

impl WepClient {
    pub async fn connect<D: Into<String>>(dst: D) -> Result<Self> {
        let channel = Channel::from_shared(dst.into())?.connect().await?;
        Ok(Self { channel })
    }
}


