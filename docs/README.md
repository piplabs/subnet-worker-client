## Worker Node - Architecture Overview (Master Doc)

This document is the high-level, repo-agnostic guide for implementing and operating a Subnet worker node.

### Purpose and Scope
- The worker node executes on-chain scheduled activities reliably and securely.
- Split of responsibilities:
  - Worker Client Process (WCP): on-chain orchestration, scheduling, reliability, storage brokering.
  - Worker Execution Process (WEP): partner-defined business logic (language-agnostic) over a stable gRPC protocol.

### Core Modules (conceptual)
- Scheduler: selects activities to run based on queue policies and available capacity; assigns to WEP.
- Broadcaster: submits on-chain txs (claim/heartbeat/complete/resume) with retries and gas bumping.
- TxManager: manages one nonce lane per wallet; EIP-1559 fees; replacement policy.
- EventPoller: watches on-chain events to maintain a fresh view and assist reconciliation.
- Persistence: lightweight KV (e.g., RocksDB) to track `inflight`, `tx`, `done`, `nonce` and idempotency.
- Storage Broker: mediates presigned URLs via API; WEP never holds long-lived creds.
- WEP SDKs: provide handler registry, TaskStream server bootstrap, and progress/completion helpers.

### Protocol (WCP ⇄ WEP)
- Transport: gRPC bidirectional stream (Execution.TaskStream).
- Messages (envelope): Hello/Capabilities/CapacityUpdate, TaskAssignment, Progress, Completion, Cancel/Drain.
- Versioning: semver handshake; unknown fields ignored for forward-compat.

### Activity Specs
- Stable, static specs define each activity’s inputs/outputs (e.g., YAML). WCP maps on-chain ABI → TaskAssignment according to the spec; WEP handlers accept typed inputs and return result references (R2 keys) or small inline payloads.

### End-to-End Flow
1) WCP polls/observes queues → claims activity on-chain.
2) WCP assigns TaskAssignment to WEP (presigned URLs included); tracks `inflight`.
3) WEP runs handler, streams Progress; WCP translates to heartbeats.
4) WEP sends Completion; WCP completes on-chain and (if finalized) resumes workflow; marks `done`.

### Failure and Recovery
- Reconciliation loop: bounded scan window (by lease/TTL) over TaskQueue/WorkflowEngine events + `getWorkerActivities(wallet)` to rebuild `inflight` and `done` idempotently.
- DB wipe: recover from on-chain state; checkpoints stored off-chain via `checkpoint_ref`.

### Capacity and Scheduling
- WEP announces capabilities (max_concurrency, tags); optionally class-based capacities (cpu/gpu/io) and credits.
- WCP never oversubscribes; respects credits and queue priorities.

### Security Model
- Wallet keys live only in WCP.
- WEP accesses data only through presigned URLs; cross-host uses mTLS.

### Observability and Ops
- Logs include `activity_id` and `run_id` for traceability.
- Metrics to track: claims, heartbeats, completes, gas bumps, expiries, reconciliation repairs.
- Health/readiness endpoints on both WCP and WEP; runbooks for nonce conflicts, expiry, DB recovery.

### Configuration and Version Gates
- Config surfaces: RPC endpoints, contract addresses, queues, deadlines, bump %, API endpoints.
- Contract ↔ WCP semver gate; WCP refuses to run on mismatch.
- WCP ↔ WEP proto semver handshake; SDKs pin to proto versions.

### Deployment Patterns
- Same-host: WCP and WEP on one node (localhost gRPC) for simplicity.
- Cross-host: VPC + mTLS; service discovery; keep latency low.
- Scale-out: one wallet per WCP; multiple WEP instances per node is fine; add nodes to scale horizontally.

### Pointers to Detailed Docs
- Global worker architecture: see `subnet-worker/docs/WORKER_ARCHITECTURE.md`.
- WCP SoT (implementation details): `docs/WCP_SOT.md`.
- WEP SDK (Python) quickstart: `docs/WEP_SDK.md`.
- Execution proto (SoT): `proto/execution/v1/execution.proto`.
- Activity specs: `activity-specs/`.


