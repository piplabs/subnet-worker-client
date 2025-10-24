## WEP SDK (Python) - Quickstart

- SDK package: `sdks/python/poseidon_wep_sdk_pkg`
- Example app: `python-wep-ex/` (imports the SDK like a node maintainer would)

### Install (local)
- Create venv, then from repo root: `pip install -e sdks/python/poseidon_wep_sdk_pkg`
- For REST API: `pip install fastapi uvicorn`

### Implement a handler (in your app repo)
```python
from poseidon_wep_sdk_pkg.registry import task, bind_spec_from_yaml
from poseidon_wep_sdk_pkg.types import TaskAssignment, Completion

bind_spec_from_yaml("video.preprocess", "1.0.0", "shared/workflows/video_v1_0_0.yaml")

@task("video.preprocess", "1.0.0")
def preprocess(assign: TaskAssignment) -> Completion:
    return Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="SUCCESS", result_ref=f"{assign.upload_prefix}/result.json")
```

### Run example WEP

#### REST API (Default)
- `python python-wep-ex/main.py`
- WEP listens on `127.0.0.1:8080` by default
- Endpoints:
  - `POST /tasks/{id}/assign` - Accept task assignments
  - `GET /tasks/{id}/status` - Report task progress
  - `GET /health` - Health check endpoint

#### gRPC (Legacy)
- `python python-wep-ex/main_grpc.py`
- WEP listens on `127.0.0.1:7070` by default

### Run WCP side-by-side
- `ENVIRONMENT=local cargo run --bin subnet-wcp`
- WCP will use REST API by default (port 8080)
- To use gRPC instead: Set `WEP_ENDPOINT=http://127.0.0.1:7070`

### Notes
- Validation: If required inputs are missing or no handler is registered for a received activity, the SDK returns an ERROR Completion.
- Spec binding: Use `bind_spec_from_yaml(kind, version, path)` to enable validation at runtime.
- Proto: `proto/execution/v1/execution.proto` (SoT).
- In production, publish the SDK to PyPI; partner apps depend on it and implement handlers in their own repos.
