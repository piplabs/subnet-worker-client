use anyhow::Result;
use alloy::providers::Provider;
use tracing::info;

pub struct ChainClient<P: Provider + Clone + Send + Sync + 'static> {
    pub rpc_url: P,
    pub wallet_address: String,
}

impl<P: Provider + Clone + Send + Sync + 'static> ChainClient<P> {
    pub async fn new(provider: P, wallet_address: String) -> Result<Self> {
        info!(wallet = %wallet_address, "chain client ready");
        Ok(Self { rpc_url: provider, wallet_address })
    }
}

pub mod txmgr {
    use alloy::rpc::types::TransactionRequest;
    use anyhow::Result;

    pub struct TxPolicy { pub gas_bump_percent: u32 }

    pub fn apply_policy(_tx: &mut TransactionRequest, _policy: &TxPolicy) -> Result<()> {
        // TODO: set EIP-1559 fields and bumping strategy hooks
        Ok(())
    }

    pub async fn submit_with_retry() -> Result<()> { Ok(()) }
}

pub mod task_queue {
    use alloy::primitives::Address;
    use alloy::providers::Provider;
    use anyhow::Result;

    pub async fn poll_activity<P: Provider + Clone + Send + Sync + 'static>(_provider: &P, _task_queue_addr: Address, _queue_name: &str) -> Result<Option<[u8;32]>> {
        // Stubbed: return None; real on-chain call is disabled for MVP
        Ok(None)
    }
}

pub mod control_plane {
    use alloy::primitives::Address;
    use alloy::providers::Provider;
    use anyhow::Result;
    use alloy_sol_types::sol;

    // Minimal contract binding for SubnetControlPlane used by WCP
    sol! {
        #[sol(rpc)]
        contract SubnetControlPlane {
            function isWorkerActive(address worker) external view returns (bool);
        }
    }

    pub async fn is_worker_active<P: Provider + Clone + Send + Sync + 'static>(provider: &P, scp_addr: Address, worker: Address) -> Result<bool> {
        let contract = SubnetControlPlane::new(scp_addr, provider.clone());
        let is_active = contract.isWorkerActive(worker).call().await?;
        Ok(is_active)
    }

    pub async fn get_protocol_version<P: Provider + Clone + Send + Sync + 'static>(_provider: &P, _scp_addr: Address) -> Result<String> {
        // Stubbed: return current version from contract for MVP
        Ok("0.2.0".to_string())
    }
}


