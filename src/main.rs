use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};
use subnet_wcp_config::WcpConfig;
use subnet_wcp_persistence::KvStore;
use subnet_wcp_scheduler::Scheduler;
use subnet_wcp_broadcaster::{Broadcaster, BroadcasterConfig};
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
    let bc_cfg = BroadcasterConfig {
        wep_endpoint_http: cfg.wep_endpoint_http.clone().unwrap_or_else(|| "http://127.0.0.1:7070".to_string()),
        max_inflight: cfg.scheduler.max_inflight,
    };
    let bc = Broadcaster::new_with_cfg(store.clone(), provider.clone(), bc_cfg);
    let broadcaster = tokio::spawn(async move { let _ = bc.run_claim_loop().await; });

    let _ = tokio::join!(poller, broadcaster);

    Ok(())
}


