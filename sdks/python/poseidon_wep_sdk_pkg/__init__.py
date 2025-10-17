# Convenience imports/exports to smooth IDEs and runtime
from . import generated  # noqa: F401
from .registry import task  # noqa: F401
from .types import TaskAssignment, Completion, InputDescriptor  # noqa: F401

# Expose top-level package name expected by grpc_tools generated code
# so that `from execution.v1 import execution_pb2` works without PYTHONPATH hacks.
import sys as _sys
from .generated import execution as _exec_pkg  # type: ignore
_sys.modules.setdefault("execution", _exec_pkg)

__all__ = [
    "task",
    "TaskAssignment",
    "Completion",
    "InputDescriptor",
]


