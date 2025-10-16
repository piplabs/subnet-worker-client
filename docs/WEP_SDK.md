## WEP SDK (Python) - Quickstart

- SDK package: `sdks/python/poseidon_wep_sdk_pkg`
- Example app: `python-wep-ex/` (imports the SDK like a node maintainer would)

### Install (local)
- Create venv, then from repo root: `pip install -e sdks/python/poseidon_wep_sdk_pkg`

### Implement a handler (in your app repo)
```python
from poseidon_wep_sdk_pkg.registry import task
from poseidon_wep_sdk_pkg.types import TaskAssignment, Completion

@task("video.preprocess", "1.1.0")
def preprocess(assign: TaskAssignment) -> Completion:
    return Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="SUCCESS", result_ref=f"{assign.upload_prefix}/result.json")
```

### Run example WEP
- `python python-wep-ex/main.py`
- WEP listens on `127.0.0.1:7070` by default.

### Run WCP side-by-side
- From repo root: `ENVIRONMENT=local cargo run -p subnet-wcp`

### Notes
- Proto: `proto/execution/v1/execution.proto` (SoT). SDK will generate stubs against this later.
- In production, publish the SDK to PyPI; partner apps depend on it and implement handlers in their own repos.
