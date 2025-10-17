use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};
use subnet_wcp_config::WcpConfig;
mod components;
use subnet_wcp_persistence::KvStore;
use components::poller::Poller;
use components::assigner::Assigner;
use alloy::providers::ProviderBuilder;
use alloy::primitives::Address;
use subnet_wcp_chain::control_plane as scp;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    tracing::info!("Starting Subnet Worker Client Process (WCP)");

    let cfg = WcpConfig::from_env()?;
    let store = KvStore::open("./wcp.db")?;
    let provider = ProviderBuilder::new().on_http(cfg.ethereum.rpc_url.parse()?);
    // Contract protocol semver check
    let scp_addr: Address = cfg.ethereum.subnet_control_plane_address.parse()?;
    let ver = scp::get_protocol_version(&provider, scp_addr).await?;
    let allowed_min = &cfg.protocol.contract_min;
    let allowed_max = &cfg.protocol.contract_max;
    if !semver_in_range(&ver, allowed_min, allowed_max) {
        tracing::error!(onchain=%ver, min=%allowed_min, max=%allowed_max, "Contract protocol version out of supported range");
        anyhow::bail!("contract protocol version incompatible");
    }

    // Spawn poller
    let poll_interval = cfg.scheduler.poll_interval;
    let queue_name = cfg.scheduler.queue_name.clone();
    let task_queue_addr: Address = cfg.ethereum.task_queue_address.parse()?;
    let poll = Poller::new(store.clone(), poll_interval, queue_name, provider.clone(), task_queue_addr);
    let poller = tokio::spawn(async move { let _ = poll.run().await; });

    // Spawn WEP RPC Assigner (schedules to WEP based on capacity)
    let wep_endpoint = cfg.wep_grpc_endpoint.clone().unwrap_or_else(|| "http://127.0.0.1:7070".to_string());
    let max_inflight = cfg.scheduler.max_inflight;
    let assigner = Assigner::new(store.clone(), wep_endpoint, max_inflight);
    let assigner_task = tokio::spawn(async move { let _ = assigner.run().await; });

    let _ = tokio::join!(poller, assigner_task);

    Ok(())
}
fn semver_in_range(ver: &str, min: &str, max: &str) -> bool {
    use semver::Version;
    match (Version::parse(ver), Version::parse(min), Version::parse(max)) {
        (Ok(v), Ok(lo), Ok(hi)) => v >= lo && v <= hi,
        _ => false,
    }
}


