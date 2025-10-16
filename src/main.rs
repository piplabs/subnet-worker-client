use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};
use subnet_wcp_config::WcpConfig;
use subnet_wcp_persistence::KvStore;
use subnet_wcp_scheduler::Scheduler;
use subnet_wcp_broadcaster::Broadcaster;
use alloy::providers::ProviderBuilder;
use alloy::primitives::Address;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    tracing::info!("Starting Subnet Worker Client Process (WCP)");

    let cfg = WcpConfig::from_env()?;
    let store = KvStore::open("./wcp.db")?;
    let provider = ProviderBuilder::new().on_http(cfg.ethereum.rpc_url.parse()?);

    // Spawn poller
    let poll_interval = cfg.scheduler.poll_interval;
    let queue_name = cfg.scheduler.queue_name.clone();
    let task_queue_addr: Address = cfg.ethereum.task_queue_address.parse()?;
    let scheduler = Scheduler::new(store.clone(), poll_interval, queue_name, provider.clone(), task_queue_addr);
    let poller = tokio::spawn(async move { let _ = scheduler.run().await; });

    // Spawn broadcaster (claim placeholder)
    let bc = Broadcaster::new(store.clone(), provider.clone());
    let broadcaster = tokio::spawn(async move { let _ = bc.run_claim_loop().await; });

    let _ = tokio::join!(poller, broadcaster);

    Ok(())
}


