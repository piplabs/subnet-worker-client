from dataclasses import dataclass
from typing import List, Optional

@dataclass
class InputDescriptor:
    name: str
    media_type: str
    ref: Optional[str] = None
    inline_json: Optional[str] = None
    inline_bytes: Optional[bytes] = None

@dataclass
class TaskAssignment:
    activity_id: str
    workflow_instance_id: str
    run_id: str
    task_kind: str
    task_version: str
    inputs: List[InputDescriptor]
    upload_prefix: str
    soft_deadline_unix: int
    heartbeat_interval_s: int

@dataclass
class Progress:
    activity_id: str
    run_id: str
    percent: int
    message: str = ""
    checkpoint_ref: Optional[str] = None

@dataclass
class Completion:
    activity_id: str
    run_id: str
    status: str
    result_ref: Optional[str] = None
    result_inline: Optional[bytes] = None
    error: Optional[str] = None
