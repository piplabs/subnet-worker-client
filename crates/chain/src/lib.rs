use anyhow::Result;
use alloy::providers::Provider;
use alloy::primitives::Address;
use tracing::info;

pub struct ChainClient<P: Provider + Clone + Send + Sync + 'static> {
    pub provider: P,
    pub wallet_address: String,
}

impl<P: Provider + Clone + Send + Sync + 'static> ChainClient<P> {
    pub async fn new(provider: P, wallet_address: String) -> Result<Self> {
        info!(wallet = %wallet_address, "chain client ready");
        Ok(Self { provider, wallet_address })
    }
}

pub mod txmgr {
    use alloy::rpc::types::TransactionRequest;
    use anyhow::Result;
    use tracing::warn;

    pub struct TxPolicy { pub gas_bump_percent: u32 }

    pub fn apply_policy(tx: &mut TransactionRequest, _policy: &TxPolicy) -> Result<()> {
        // TODO: set EIP-1559 fields and bumping strategy hooks
        Ok(())
    }

    pub async fn submit_with_retry() -> Result<()> {
        // TODO: retries/backoff
        warn!("tx submit_with_retry not implemented");
        Ok(())
    }
}


