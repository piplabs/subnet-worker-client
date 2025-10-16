use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};
use subnet_wcp_config::WcpConfig;
use subnet_wcp_persistence::KvStore;
use subnet_wcp_scheduler::Scheduler;
use std::time::Duration;
use alloy::providers::ProviderBuilder;
use alloy::primitives::Address;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    tracing::info!("Starting Subnet Worker Client Process (WCP)");

    // Load config from env (WCP__*) and configs/{ENVIRONMENT}.toml
    let cfg = WcpConfig::from_env()?;

    // Open local store
    let store = KvStore::open("./wcp.db")?;

    // Build provider
    let provider = ProviderBuilder::new().on_http(cfg.ethereum.rpc_url.parse()?);

    // TODO: on-start check registration via SubnetControlPlane.isWorkerActive(cfg.ethereum.wallet_address)

    // Start poller that enqueues claim jobs
    let poll_interval = cfg.scheduler.poll_interval;
    let queue_name = cfg.scheduler.queue_name.clone();
    let task_queue_addr: Address = cfg.ethereum.task_queue_address.parse()?;
    let scheduler = Scheduler::new(store, poll_interval, queue_name, provider, task_queue_addr);

    scheduler.run().await?;

    Ok(())
}


