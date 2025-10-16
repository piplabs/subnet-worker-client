pub mod execution_v1 {
    tonic::include_proto!("execution.v1");
}

use anyhow::Result;
use tonic::transport::Channel;
use execution_v1::{execution_client::ExecutionClient, Envelope};
use tokio::sync::mpsc::{Sender, channel};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Streaming;

pub struct WepGrpcClient {
    inner: ExecutionClient<Channel>,
}

impl WepGrpcClient {
    pub async fn connect<D: Into<String>>(dst: D) -> Result<Self> {
        let channel = Channel::from_shared(dst.into())?.connect().await?;
        Ok(Self { inner: ExecutionClient::new(channel) })
    }

    pub async fn open_task_stream(&mut self) -> Result<(Sender<Envelope>, Streaming<Envelope>)> {
        let (tx, rx) = channel::<Envelope>(32);
        let request = tonic::Request::new(ReceiverStream::new(rx));
        let response = self.inner.task_stream(request).await?;
        Ok((tx, response.into_inner()))
    }
}


