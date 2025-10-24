use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};
use subnet_wcp_config::WcpConfig;
mod components;
use subnet_wcp_persistence::KvStore;
use components::poller::Poller;
use components::assigner::Assigner;
use alloy::providers::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use components::broadcaster::Broadcaster as ChainBroadcaster;
use alloy::primitives::Address;
use subnet_wcp_chain::control_plane as scp;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    tracing::info!("Starting Subnet Worker Client Process (WCP)");

    let cfg = WcpConfig::from_env()?;
    let store = KvStore::open("./wcp.db")?;
    // Provider with wallet for tx signing
    let signer: PrivateKeySigner = cfg.ethereum.wallet_private_key.parse()?;
    let provider = ProviderBuilder::new().wallet(signer).connect_http(cfg.ethereum.rpc_url.parse()?);
    // Contract protocol semver check
    let scp_addr: Address = cfg.ethereum.subnet_control_plane_address.parse()?;
    let ver = scp::get_protocol_version(&provider, scp_addr).await?;
    let allowed_min = &cfg.protocol.contract_min;
    let allowed_max = &cfg.protocol.contract_max;
    if !semver_in_range(&ver, allowed_min, allowed_max) {
        tracing::error!(onchain=%ver, min=%allowed_min, max=%allowed_max, "Contract protocol version out of supported range");
        anyhow::bail!("contract protocol version incompatible");
    }

    // Registration gate: ensure this worker is active
    let worker_addr: Address = cfg.ethereum.wallet_address.parse()?;
    let active = scp::is_worker_active(&provider, scp_addr, worker_addr).await?;
    if !active {
        tracing::error!(worker=%cfg.ethereum.wallet_address, "Worker is not active. Refusing to start.");
        anyhow::bail!("worker not active on SubnetControlPlane");
    }

    // Spawn poller
    let poll_interval = cfg.scheduler.poll_interval;
    let queue_name = cfg.scheduler.queue_name.clone();
    let task_queue_addr: Address = cfg.ethereum.task_queue_address.parse()?;
    let poll = Poller::new(store.clone(), poll_interval, queue_name, provider.clone(), task_queue_addr);
    let poller = tokio::spawn(async move { let _ = poll.run().await; });

    // Spawn WEP Assigner (REST API by default)
    let wep_endpoint = cfg.wep_endpoint.clone()
        .or_else(|| std::env::var("WEP_ENDPOINT").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let max_inflight = cfg.scheduler.max_inflight;
    if cfg.dev_mode.unwrap_or(false) {
        std::env::set_var("DEV_MOCK_ASSIGNER", "1");
    }
    let assigner = Assigner::new(store.clone(), wep_endpoint, max_inflight);
    let assigner_task = tokio::spawn(async move { let _ = assigner.run().await; });

    // Spawn Broadcaster (chain tx pipeline skeleton)
    let task_queue_addr_bc: Address = cfg.ethereum.task_queue_address.parse()?;
    let workflow_engine_addr_bc: Address = cfg.ethereum.workflow_engine_address.parse()?;
    let chain_bc = ChainBroadcaster::new(store.clone(), provider.clone(), task_queue_addr_bc, workflow_engine_addr_bc);
    let broadcaster_task = tokio::spawn(async move { let _ = chain_bc.run().await; });

    let _ = tokio::join!(poller, assigner_task, broadcaster_task);

    Ok(())
}
fn semver_in_range(ver: &str, min: &str, max: &str) -> bool {
    use semver::Version;
    match (Version::parse(ver), Version::parse(min), Version::parse(max)) {
        (Ok(v), Ok(lo), Ok(hi)) => v >= lo && v <= hi,
        _ => false,
    }
}


