use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub wallet_private_key: String,
    pub wallet_address: String,
    pub workflow_engine_address: String,
    pub task_queue_address: String,
    pub multicall3_address: String,
    pub subnet_control_plane_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SubnetApiConfig {
    pub grpc_endpoint: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SchedulerConfig {
    #[serde(with = "humantime_serde")]
    pub poll_interval: Duration,
    pub max_inflight: usize,
    pub queue_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TxPolicyConfig {
    pub gas_bump_percent: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WcpConfig {
    pub ethereum: EthereumConfig,
    pub subnet_api: SubnetApiConfig,
    pub scheduler: SchedulerConfig,
    pub tx_policy: TxPolicyConfig,
    pub wep_endpoint_http: Option<String>,
}

impl WcpConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
        let file = format!("configs/{}.toml", env);
        let mut c = config::Config::builder()
            .add_source(config::File::with_name(&file).required(false))
            .add_source(config::Environment::with_prefix("WCP").separator("__"))
            .build()?;
        c.try_deserialize().map_err(Into::into)
    }
}


