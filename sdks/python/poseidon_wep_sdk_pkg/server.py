from __future__ import annotations
import asyncio
from typing import AsyncIterator, Optional, List, Any

import grpc

from .registry import get_handler, get_spec
from .config import load_wep_config
from .types import TaskAssignment as SdkTaskAssignment, InputDescriptor as SdkInput, Completion as SdkCompletion
from .generated.execution.v1 import execution_pb2 as pb
from .generated.execution.v1 import execution_pb2_grpc as pbg

# Type aliases for readability; ignore static analyzers if they cannot resolve generated symbols
PBCompletion = Any  # type: ignore[assignment]
PBTaskAssignment = Any  # type: ignore[assignment]
PBEnvelope = Any  # type: ignore[assignment]


def _pb_to_sdk_assignment(a: PBTaskAssignment) -> SdkTaskAssignment:
    inputs = [SdkInput(name=i.name, media_type=i.media_type, ref=i.ref, inline_json=i.inline_json, inline_bytes=i.inline_bytes) for i in a.inputs]
    return SdkTaskAssignment(
        activity_id=a.activity_id,
        workflow_instance_id=a.workflow_instance_id,
        run_id=a.run_id,
        task_kind=a.task_kind,
        task_version=a.task_version,
        inputs=inputs,
        upload_prefix=a.upload_prefix,
        soft_deadline_unix=a.soft_deadline_unix,
        heartbeat_interval_s=a.heartbeat_interval_s,
    )


def _sdk_to_pb_completion(c: SdkCompletion) -> PBCompletion:
    return pb.Completion(activity_id=c.activity_id, run_id=c.run_id, status=c.status, result_ref=c.result_ref or "", result_inline=c.result_inline or b"", error=c.error or "")  # type: ignore[attr-defined]


class ExecutionServiceServicer(pbg.ExecutionServicer):
    def __init__(self, proto_min: str = "1.0.0", proto_max: str = "1.0.0"):
        self.proto_min = proto_min
        self.proto_max = proto_max

    async def TaskStream(self, request_iterator: AsyncIterator[PBEnvelope], context: grpc.aio.ServicerContext):  # type: ignore
        # Simple state: after receiving Assign, run handler and yield Completion
        print("WEP: TaskStream opened")
        async for env in request_iterator:
            which = env.WhichOneof("msg")
            if which == "hello":
                print("WEP: received hello")
                # Reply with hello_ack selecting our supported range (simple echo for MVP)
                out = pb.Envelope()  # type: ignore[attr-defined]
                out.hello_ack.CopyFrom(pb.Version(min=self.proto_min, max=self.proto_max))  # type: ignore[attr-defined]
                yield out
            elif which == "capabilities":
                print(f"WEP: received capabilities max_concurrency={env.capabilities.max_concurrency} tags={list(env.capabilities.tags)}")
            elif which == "assign":
                assign = env.assign
                print(f"WEP: received assignment activity_id={assign.activity_id}")
                handler = get_handler(assign.task_kind, assign.task_version)
                if handler is None:
                    comp = pb.Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="ERROR", error=f"No handler for {assign.task_kind}:{assign.task_version}")  # type: ignore[attr-defined]
                else:
                    # Validate against spec if available
                    spec = get_spec(assign.task_kind, assign.task_version)
                    if spec and "inputs" in spec:
                        required_inputs = {i.get("name"): i for i in spec["inputs"]}
                        provided_inputs = {i.name: i for i in assign.inputs}
                        missing = [n for n in required_inputs.keys() if n not in provided_inputs]
                        if missing:
                            comp = pb.Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="ERROR", error=f"Missing required inputs: {missing}")  # type: ignore[attr-defined]
                            out = pb.Envelope()  # type: ignore[attr-defined]
                            out.completion.CopyFrom(comp)
                            print(f"WEP: sending completion activity_id={comp.activity_id} status={comp.status}")
                            yield out
                            continue
                    sdk_assign = _pb_to_sdk_assignment(assign)
                    try:
                        sdk_comp: SdkCompletion = await asyncio.to_thread(handler, sdk_assign)
                        # Optional output validation placeholder: check presence of result_ref
                        if spec and spec.get("outputs"):
                            if not (sdk_comp.result_ref or sdk_comp.result_inline):
                                raise ValueError("Handler did not produce result_ref or inline result as required by spec")
                        comp = _sdk_to_pb_completion(sdk_comp)
                    except Exception as e:
                        print(f"WEP: handler error: {e}")
                        comp = pb.Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="ERROR", error=str(e))  # type: ignore[attr-defined]
                out = pb.Envelope()  # type: ignore[attr-defined]
                out.completion.CopyFrom(comp)
                print(f"WEP: sending completion activity_id={comp.activity_id} status={comp.status}")
                yield out


async def serve(host: str = "127.0.0.1", port: int = 7070, max_concurrency: int = 4, proto_min: str = "1.0.0", proto_max: str = "1.0.0"):
    server = grpc.aio.server(maximum_concurrent_rpcs=max_concurrency, options=[('grpc.so_reuseport', 0)])
    pbg.add_ExecutionServicer_to_server(ExecutionServiceServicer(proto_min=proto_min, proto_max=proto_max), server)
    server.add_insecure_port(f"{host}:{port}")
    await server.start()
    print(f"WEP: listening on {host}:{port}, max_concurrency={max_concurrency}")
    await server.wait_for_termination()


class WepServer:
    def __init__(self, host: str = "127.0.0.1", port: int = 7070, max_concurrency: int = 4, tags: Optional[List[str]] = None, proto_min: str = "1.0.0", proto_max: str = "1.0.0"):
        self.host = host
        self.port = port
        self.max_concurrency = max_concurrency
        self.tags = tags or []
        self.proto_min = proto_min
        self.proto_max = proto_max

    async def start(self):
        await serve(self.host, self.port, self.max_concurrency, self.proto_min, self.proto_max)
