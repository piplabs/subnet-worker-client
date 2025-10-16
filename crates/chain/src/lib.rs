use anyhow::Result;
use alloy::providers::Provider;
use alloy::primitives::{Address, B256, Bytes, U256};
use tracing::{info, warn};

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

pub mod task_queue {
    use alloy::dyn_abi::{DynSolType, DynSolValue};
    use alloy::primitives::{Address, B256};
    use alloy::providers::Provider;
    use anyhow::Result;

    pub async fn poll_activity<P: Provider + Clone + Send + Sync + 'static>(
        provider: &P,
        task_queue_addr: Address,
        queue_name: &str,
    ) -> Result<Option<B256>> {
        // function pollActivity(string,uint16) returns ((...), bool)
        let func = alloy::dyn_abi::Function::new(
            "pollActivity",
            vec![DynSolType::String, DynSolType::Uint(16)],
            vec![
                DynSolType::Tuple(vec![
                    DynSolType::FixedBytes(32), // activityId
                    DynSolType::FixedBytes(32), // workflowInstanceId
                    DynSolType::Address,
                    DynSolType::Bytes,
                    DynSolType::Bytes,
                    DynSolType::Bytes,
                    DynSolType::Uint(256),
                    DynSolType::Uint(256),
                    DynSolType::Address,
                    DynSolType::Address,
                    DynSolType::Uint(256),
                    DynSolType::Uint(256),
                    DynSolType::Uint(8),
                    DynSolType::Bool,
                    DynSolType::Bool,
                    DynSolType::Bool,
                ]),
                DynSolType::Bool,
            ],
        );
        let calldata = func.encode_input(&[DynSolValue::String(queue_name.into()), DynSolValue::Uint(16u8.into())])?;
        let result = provider.call(&task_queue_addr, calldata.into(), None).await?;
        let decoded = func.decode_output(result.as_ref())?;
        if let DynSolValue::Tuple(vals) = &decoded[0] {
            if let DynSolValue::FixedBytes(id_bytes) = &vals[0] {
                let has = if let alloy::dyn_abi::DynSolValue::Bool(b) = &decoded[1] { *b } else { false };
                if has {
                    let mut id_arr = [0u8;32];
                    id_arr.copy_from_slice(id_bytes);
                    return Ok(Some(B256::from(id_arr)));
                }
            }
        }
        Ok(None)
    }
}

pub mod control_plane {
    use alloy::dyn_abi::{DynSolType, DynSolValue};
    use alloy::primitives::Address;
    use alloy::providers::Provider;
    use anyhow::Result;

    pub async fn is_worker_active<P: Provider + Clone + Send + Sync + 'static>(
        provider: &P,
        scp_addr: Address,
        worker: Address,
    ) -> Result<bool> {
        let func = alloy::dyn_abi::Function::new(
            "isWorkerActive",
            vec![DynSolType::Address],
            vec![DynSolType::Bool],
        );
        let calldata = func.encode_input(&[DynSolValue::Address(worker)])?;
        let result = provider.call(&scp_addr, calldata.into(), None).await?;
        let decoded = func.decode_output(result.as_ref())?;
        if let DynSolValue::Bool(b) = decoded[0] { Ok(b) } else { Ok(false) }
    }
}


