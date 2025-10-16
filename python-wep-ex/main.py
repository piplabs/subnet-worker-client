import asyncio
from poseidon_wep_sdk_pkg.registry import task
from poseidon_wep_sdk_pkg.types import TaskAssignment, Completion
from poseidon_wep_sdk_pkg.runner import run_server

@task("video.preprocess", "1.1.0")
def preprocess(assign: TaskAssignment) -> Completion:
    return Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="SUCCESS", result_ref=f"{assign.upload_prefix}/preprocess/result.json")

if __name__ == "__main__":
    asyncio.run(run_server())
