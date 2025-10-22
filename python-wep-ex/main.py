import asyncio
import os
import time
import argparse
from poseidon_wep_sdk_pkg.registry import task, bind_spec_from_yaml
from poseidon_wep_sdk_pkg.types import TaskAssignment, Completion
from poseidon_wep_sdk_pkg.runner import run_server

# Bind spec (mapping video workflow step to handler kind/version)
base_dir = os.path.dirname(__file__)
spec_path = os.path.abspath(os.path.join(base_dir, "..", "shared", "workflows", "video_v1_1_0.yaml"))
bind_spec_from_yaml("video.preprocess", "1.1.0", spec_path)

@task("video.preprocess", "1.1.0")
def preprocess(assign: TaskAssignment) -> Completion:
    
    #DO STUFF HERE
    print(f"preprocessing {assign.activity_id}")

    time.sleep(10)

    return Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="SUCCESS", result_ref=f"{assign.upload_prefix}/preprocess/result.json")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", dest="config", default=os.environ.get("WEP_CONFIG"), help="Path to WEP YAML config")
    args = parser.parse_args()
    if args.config:
        asyncio.run(run_server(config_path=args.config))
    else:
        # Default to local config inside this repo if present
        default_cfg = os.path.join(base_dir, "wep.local.yaml")
        if os.path.exists(default_cfg):
            asyncio.run(run_server(config_path=default_cfg))
        else:
            max_conc = int(os.environ.get("WEP_MAX_CONCURRENCY", "4"))
            asyncio.run(run_server(max_concurrency=max_conc))
