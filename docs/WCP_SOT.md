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

### Sync/Reconciliation Loop (Failure Resilience)

Assumption: activities expire within ~3 hours of full assignment. WCP maintains consistency by scanning only a sliding 3h window.

Process (runs in parallel to poller/broadcaster):
- Determine scan window: from `now - 3h` to `now` (by block timestamps or estimated blocks).
- Subscribe/scan TaskQueue and WorkflowEngine events in the window:
  - ActivityEnqueued, ActivityClaimed(worker), ActivityCompleted(worker), ActivityFailed(worker), ActivityHeartbeat.
- For each relevant event referencing our wallet:
  - Reconstruct `inflight:{activity_id}` if missing.
  - Fetch `getActivity(activity_id)`; if `isCompleted`, write `done:{activity_id}` and clean inflight.
  - If claimed but not yet completed and not expired: ensure a claim job or inflight record exists; resume heartbeats scheduling.
  - If expired: ensure local cleanup; do not attempt heartbeat/complete.
- Also query `getWorkerActivities(wallet)` for direct enumeration to double-check inflight set.
- Write a `last_scan_block` checkpoint to avoid reprocessing; tolerate replays (idempotent writes).

Rationale:
- Limits on-chain scans to a bounded recent window; tolerates WCP crashes.
- Idempotent reconstruction ties local state to on-chain truth; minimizes reliance on local DB persistence.
- Complements the real-time poller so we don’t miss items due to temporary outages.

### MVP Implementation Plan

1) Registration & Startup
- At WCP start, call `SubnetControlPlane.isWorkerActive(wallet)`; if false, log and exit (MVP) or attempt registration later.
- Log config and contract addresses; sanity-check RPC.

2) Poller → Claim Jobs
- Use `TaskQueue.pollActivity(queue, 0)` every `scheduler.poll_interval`.
- On hasActivity, write `claim_job:{activity_id}` JSON into RocksDB.
- Add `scan_prefix("claim_job:")` utility (done) to verify enqueued jobs.

3) Broadcaster (claim-only MVP)
- Read a `claim_job:*`, build `claimActivity(activity_id)` tx, submit via TxManager.
- Record `tx:{activity_id}` with nonce/tx_hash, mark inflight on success.
- Confirmer watches receipts; on success, mark inflight and proceed to assignment (future step).

4) WEP SDK + Example
- Run example WEP at 127.0.0.1:7070.
- WCP assignment path (next iteration): when claim is confirmed/assigned, build TaskAssignment and deliver to WEP.

5) Reconciliation Loop (3h)
- Scan TaskQueue/WorkflowEngine events within last 3h and reconcile against local DB.
- Use `getWorkerActivities(wallet)` to enumerate inflight directly.
- Rebuild missing `inflight:*` and mark completed/expired appropriately.

6) Testing E2E (MVP)
- Start WCP with ENVIRONMENT=local; verify claim jobs appear in RocksDB via `scan_prefix` tooling.
- (Optional) Integrate a simple broadcaster to confirm claims on-chain.
- Later: wire assignment to WEP and end-to-end run preprocess handler.

Milestones
- M1: Poller writes claim jobs; DB inspection shows entries.
- M2: Broadcaster submits claims; receipts confirm; DB updated.
- M3: WEP assignment delivery; preprocess task completes; completion path queued.
- M4: Reconciliation loop validates/repairs inflight state after restarts.

### E2E MVP (gRPC WCP↔WEP, stubbed chain)

Goal: WCP assigns `video.preprocess` to WEP over gRPC; WEP processes and returns Completion; WCP marks done (chain stubbed).

- Protocol: `proto/execution/v1/execution.proto` (TaskStream)
- Activity spec: `activity-specs/video.preprocess.yaml`

Implementation Plan
- WEP server (Python grpc.aio): implement TaskStream, Hello/Capabilities/CapacityUpdate, accept TaskAssignment, run handler, send Completion
- WCP assigner: open TaskStream to WEP, send HelloAck + Capabilities handshake, on claim_job enqueue TaskAssignment, listen for Completion, write `done:{activity_id}`
- Chain stubs: assume worker registered; skip real claim/complete. Treat `claim_job:*` as ready-to-assign for MVP.

Status
- gRPC client stubs generated (Rust): `crates/rpc`
- Activity spec in place
- SDK skeleton present (Python)
- Next: implement Python grpc.aio server; implement WCP assigner loop using `WepGrpcClient::open_task_stream`

Next Steps
1) Build WEP server with TaskStream and a handler for `video.preprocess`
2) Extend WCP to send Hello, Capabilities, and TaskAssignment, and handle Completion
3) Validate end-to-end flow locally (ENVIRONMENT=local), then iterate on capacity credits and real chain paths
