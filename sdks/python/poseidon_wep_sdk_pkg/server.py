import asyncio
from typing import AsyncIterator

import grpc

from .registry import get_handler
from .types import TaskAssignment as SdkTaskAssignment, InputDescriptor as SdkInput, Completion as SdkCompletion
from .generated.execution.v1 import execution_pb2 as pb
from .generated.execution.v1 import execution_pb2_grpc as pbg

def _pb_to_sdk_assignment(a: pb.TaskAssignment) -> SdkTaskAssignment:
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

def _sdk_to_pb_completion(c: SdkCompletion) -> pb.Completion:
    return pb.Completion(activity_id=c.activity_id, run_id=c.run_id, status=c.status, result_ref=c.result_ref or "", result_inline=c.result_inline or b"", error=c.error or "")

class ExecutionServiceServicer(pbg.ExecutionServicer):
    async def TaskStream(self, request_iterator: AsyncIterator[pb.Envelope], context: grpc.aio.ServicerContext):  # type: ignore
        # Simple state: after receiving Assign, run handler and yield Completion
        async for env in request_iterator:
            if env.HasField("assign"):
                assign = env.assign
                handler = get_handler(assign.task_kind, assign.task_version)
                if handler is None:
                    comp = pb.Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="ERROR", error=f"No handler for {assign.task_kind}:{assign.task_version}")
                else:
                    sdk_assign = _pb_to_sdk_assignment(assign)
                    try:
                        sdk_comp: SdkCompletion = await asyncio.to_thread(handler, sdk_assign)
                        comp = _sdk_to_pb_completion(sdk_comp)
                    except Exception as e:
                        comp = pb.Completion(activity_id=assign.activity_id, run_id=assign.run_id, status="ERROR", error=str(e))
                out = pb.Envelope()
                out.completion.CopyFrom(comp)
                yield out

async def serve(host: str = "127.0.0.1", port: int = 7070):
    server = grpc.aio.server(options=[('grpc.so_reuseport', 0)])
    pbg.add_ExecutionServicer_to_server(ExecutionServiceServicer(), server)
    server.add_insecure_port(f"{host}:{port}")
    await server.start()
    await server.wait_for_termination()

class WepServer:
    def __init__(self, host: str = "127.0.0.1", port: int = 7070, max_concurrency: int = 4, tags: list[str] = None):
        self.host = host
        self.port = port
        self.max_concurrency = max_concurrency
        self.tags = tags or []

    async def start(self):
        await serve(self.host, self.port)
