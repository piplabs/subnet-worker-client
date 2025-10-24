## Worker Client Process (WCP) - Source of Truth

### Purpose
- On-chain orchestration for workers on a single, vertically scaled node: registration checks, polling queues, enqueuing claim jobs, heartbeats, completion, resume (future).
- Mediate storage access via subnet-api (presigned URLs) for WEP.

### Current MVP Scope
- Config loader: `configs/{ENVIRONMENT}.toml` + `WCP__*` env overrides.
- Persistence: RocksDB `wcp.db` with keys: `claim_job:{activity_id}`, `inflight:{id}`, `tx:{id}`, `done:{id}`, `nonce:last`.
- Chain provider: alloy HTTP provider.
- Poller: calls `TaskQueue.pollActivity(queue, 0)` and writes `claim_job` records.
- Assigner: consumes `claim_job:*`, sends task assignment to WEP via REST API (POST /tasks/{id}/assign), polls for completion or receives webhook, writes `done:{activity_id}`; concurrent dispatch bounded by `scheduler.max_inflight`; endpoint from `wep_endpoint`.

### Config
- `[ethereum]`: `rpc_url`, `wallet_private_key`, `wallet_address`, `workflow_engine_address`, `task_queue_address`, `multicall3_address`, `subnet_control_plane_address`.
- `[subnet_api]`: `grpc_endpoint`.
- `[scheduler]`: `poll_interval`, `max_inflight`, `queue_name`.
- `[tx_policy]`: `gas_bump_percent`.

### Flow (MVP)
1) Startup: load config, open RocksDB, construct alloy provider.
2) (Soon) Registration check: `SubnetControlPlane.isWorkerActive(wallet)`; exit or proceed.
3) Poll loop: `TaskQueue.pollActivity(queue, 0)`; if `hasActivity`, write `claim_job:{activity_id}` JSON.
4) Broadcaster loop (dev): read `claim_job:*`, send TaskAssignment over gRPC to WEP, await Completion, mark `done` and clean claim job. (Chain txs stubbed for now.)
5) Later: real claims/heartbeats/complete/resume; event poller integration.

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

### Chain and Finality
- The Subnet chain runs CometBFT consensus, providing instant finality (no re-orgs). This simplifies confirmer logic and reconciliation: once a transaction is included, it is finalized without probabilistic reorg risk.

### Current Progress (Dev Mode)
- Config: ENV-merged TOML; default RPC `https://devnet-proteus.psdnrpc.io`; wallet required.
- Control-plane calls: Alloy v1.0 `#[sol(rpc)]` bindings for `isWorkerActive`, `getProtocolVersion`.
- Broadcaster (dev): drains `broadcast:claim:*` and writes `inflight:*` immediately (mock claim confirm); confirm/bump loops disabled in dev.
- Assigner: opens WEP TaskStream, sends Hello/Capabilities and Assign; logs completion. Dev knobs:
  - `WCP__DEV_MODE=true` sets `DEV_MOCK_ASSIGNER=1` (synthesizes SUCCESS without WEP).
  - `DEV_SYNTH_COMPLETION=1` can synthesize SUCCESS if WEP doesn’t reply in time.
- WEP SDK (Python): grpc.aio server; spec binding; logs Hello/Capabilities/Assign/Completion.

### Known Gaps / Next Debug Steps
- Real WEP completion: ensure Assign envelopes reach WEP (stream stability, retry, and send error handling in Assigner).
- Re-enable broadcaster confirm/bump with EIP-1559 policy and nonce lane.
- Add metrics and health endpoints; structured shutdown; reconciliation loop wiring.

### How to Run (Dev)

#### REST API Mode (Default)
1. Start WEP:
   - `PYTHONPATH=sdks/python python3 -u python-wep-ex/main.py > wep.out 2>&1`
2. Seed an inflight record while WCP is stopped:
   - `cargo run --bin sim_confirm 0x<bytes32_activity_id>`
3. Start WCP:
   - `ENVIRONMENT=local RUST_LOG=info cargo run --bin subnet-wcp > wcp.out 2>&1`
4. Observe logs:
   - wep.out: "WEP: Accepted task", "WEP: Task completed successfully"
   - wcp.out: "Task assigned to WEP", "Task status update", "Task completed successfully"
5. Inspect KV (stop WCP to release DB lock):
   - `cargo run --bin kv_list done:` — should show `done:{activity_id}`
   - `cargo run --bin kv_list inflight:` — should not include that activity

#### gRPC Mode (Legacy)
1. Start WEP:
   - `PYTHONPATH=sdks/python python3 -u python-wep-ex/main_grpc.py > wep.out 2>&1`
2. Seed an inflight record while WCP is stopped:
   - `cargo run --bin sim_confirm 0x<bytes32_activity_id>`
3. Start WCP with gRPC endpoint:
   - `ENVIRONMENT=local WEP_ENDPOINT=http://127.0.0.1:7070 RUST_LOG=info cargo run --bin subnet-wcp > wcp.out 2>&1`

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

### Component Architecture (Single-Node, No Event Bus)

WCP is composed of a small set of Tokio tasks (components). Each component owns a clear responsibility and communicates via bounded mpsc channels with typed messages. There is no global fan-out event bus; point-to-point channels keep flow explicit and simple on a vertically scaled node.

Components (examples):
- Poller → emits ClaimJob
- Broadcaster (nonce lane, EIP-1559, bumping) ← Commands; → TxSubmitted/TxReplaced
- Confirmer → TxConfirmed/TxFailed; derives ActivityClaimed/Completed
- Reconciler → periodic repair from chain; writes RocksDB

Design choices:
- Bounded channels for backpressure; idempotent state transitions; explicit persistence (`claim_job:*`, `inflight:*`, `tx:*`, `done:*`).
  - Inflight tracking: write `inflight:{activity_id}` on dispatch; delete and write `done:{activity_id}` on completion.
- `select!`-driven timers for heartbeats/bump intervals alongside channel reads.
- Graceful shutdown: stop intake, drain N seconds, persist checkpoints.

Code layout (WCP):
- `src/components/poller.rs` — Poller
- `src/components/assigner.rs` — WEP RPC Assigner (gRPC TaskStream)
- `src/components/broadcaster.rs` — Chain tx pipeline (skeleton)
- `src/main.rs` — wires Poller + Assigner + Broadcaster

#### Scheduler vs Poller (clarification)

- Poller (visibility):
  - Enumerates available work via `pollActivity(...)` and/or events.
  - Writes `claim_job:{activity_id}` to RocksDB idempotently (append-only).
  - Does not consider capacity or policy.

- (Later, optional) Scheduler (policy + load):
  - Reads `claim_job:*`, current `inflight:*`, and WEP capacity/credits.
  - Applies queue priorities/limits, selects next activities to run.
  - Issues assignment commands to the WEP RPC Assigner and records `inflight:{activity_id}`.

- WEP RPC Assigner (execution bridge):
  - Maintains gRPC TaskStream(s) to the WEP and current capacity/credits.
  - Dispatches TaskAssignments only when capacity is available.
  - Receives Progress/Completion; enqueues on-chain follow-ups for the Broadcaster and updates persistence (`done:*`, `inflight:*`).

Data flow:
- Poller → RocksDB (`claim_job:*`).
- WEP RPC Assigner (currently) reads `claim_job:*`, respects capacity, dispatches to WEP, updates persistence and writes broadcaster jobs.
- (If Scheduler added later) Scheduler → WEP RPC Assigner (dispatch decisions).

Broadcaster job keys (LevelDB):
- `broadcast:claim:{activity_id}`
- `broadcast:complete:{activity_id}`
- `broadcast:resume:{instance_id}`

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

### E2E MVP (REST API WCP↔WEP, stubbed chain)

Goal: WCP assigns `video.preprocess` to WEP over REST API; WEP processes and returns completion; WCP marks done (chain stubbed).

#### Architecture Decision (October 24, 2024)
- **Switched from gRPC to REST API** due to complexity with bidirectional streaming
- Issues resolved: Stream hanging, complex async channel management, difficult debugging
- Benefits gained: Simpler code, standard HTTP tools, stateless design, better observability

#### Implementation Status
- **WEP server** (Python FastAPI): 
  - `python-wep-ex/main_rest.py` - REST API server with async task processing
  - Endpoints: `/tasks/{id}/assign`, `/tasks/{id}/status`, `/health`
  - In-memory task storage with progress tracking
- **WCP assigner** (Rust):
  - `src/components/assigner_rest.rs` - REST client using reqwest
  - Task assignment via POST, status polling with timeout
  - Base64 encoding for binary data in JSON
- **Configuration**:
  - `wep_endpoint` for REST API (default: `http://127.0.0.1:8080`)
  - `wep_grpc_endpoint` retained for backward compatibility
  - Use `cargo run --bin subnet-wcp-rest` for REST mode

#### REST API Flow
1. WCP sends task assignment: `POST /tasks/{id}/assign`
2. WEP returns 202 Accepted, processes asynchronously
3. WCP polls status: `GET /tasks/{id}/status` 
4. WEP returns progress updates (0-100%)
5. On completion, WCP marks task as done in KV store

Status: **Fully implemented and tested** - Tasks successfully flow from assignment through completion.

TODO (next):
- Implement real claim/heartbeat/complete/resume tx paths in broadcaster (EIP-1559 + bumping)
- Confirmer actor to emit ActivityClaimed/Completed, drive follow-up actions
- Registration check at startup: `SubnetControlPlane.isWorkerActive`
- Event poller for ActivityEnqueued to complement polling
- Structured shutdown across actors; metrics for queue depths/latencies
- Remove per-job YAML read in Assigner; derive `task_kind`/`task_version` from config or job metadata
- Capacity updates from WEP and policy in Assigner (beyond `max_inflight`)

Next Steps
1) Build WEP server with TaskStream and a handler for `video.preprocess`
2) Extend WCP to send Hello, Capabilities, and TaskAssignment, and handle Completion
3) Validate end-to-end flow locally (ENVIRONMENT=local), then iterate on capacity credits and real chain paths
