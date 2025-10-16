## Worker Client Process (WCP) - Source of Truth

### Purpose
- On-chain orchestration for workers: registration checks, polling queues, enqueuing claim jobs, heartbeats, completion, resume (future).
- Mediate storage access via subnet-api (presigned URLs) for WEP.

### Current MVP Scope
- Config loader: `configs/{ENVIRONMENT}.toml` + `WCP__*` env overrides.
- Persistence: RocksDB `wcp.db` with keys: `claim_job:{activity_id}`, `inflight:{id}`, `tx:{id}`, `done:{id}`, `nonce:last`.
- Chain provider: alloy HTTP provider.
- Poller: calls `TaskQueue.pollActivity(queue, 0)` and writes `claim_job` records.
- (Future) Broadcaster: consumes `claim_job:*`, builds txs (claim), batches via Multicall3, submits, confirms.

### Config
- `[ethereum]`: `rpc_url`, `wallet_private_key`, `wallet_address`, `workflow_engine_address`, `task_queue_address`, `multicall3_address`, `subnet_control_plane_address`.
- `[subnet_api]`: `grpc_endpoint`.
- `[scheduler]`: `poll_interval`, `max_inflight`, `queue_name`.
- `[tx_policy]`: `gas_bump_percent`.

### Flow (MVP)
1) Startup: load config, open RocksDB, construct alloy provider.
2) (Soon) Registration check: `SubnetControlPlane.isWorkerActive(wallet)`; exit or proceed.
3) Poll loop: `TaskQueue.pollActivity(queue, 0)`; if `hasActivity`, write `claim_job:{activity_id}` JSON.
4) (Soon) Broadcaster loop: read claim_job, create claim tx, submit, confirm, mark inflight.
5) (Later) Heartbeats, completion, resume, WEP RPC integration.

### Keys and Records
- `claim_job:{activity_id}` => `{ activity_id, queue_name, created_at_ms }` (JSON)
- `inflight:{activity_id}` => assigned worker state
- `tx:{activity_id}` => nonce, tx hash, fee params
- `done:{activity_id}` => completion summary
- `nonce:last` => last used nonce snapshot

### Contracts
- TaskQueue: `pollActivity(queue, 0)`, `claimActivity(id)`, heartbeats, complete/fail.
- SubnetControlPlane: `isWorkerActive(addr)`, capacity tracking, worker stats.
- WorkflowEngine: `resumeWorkflow(instance)` (future).

### Reliability
- If DB wiped, rehydrate via `getWorkerActivities(wallet)` + `getActivity(id)`; rebuild `inflight` and re-enqueue work (see global architecture doc for details).

### Next Steps
- Implement registration check and broadcaster claim path.
- Add event poller for ActivityEnqueued to complement `pollActivity`.
- Integrate WEP RPC and storage broker.
