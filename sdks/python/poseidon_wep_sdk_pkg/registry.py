from typing import Callable, Dict, Tuple, Optional
import yaml
from dataclasses import dataclass

_handlers: Dict[Tuple[str, str], Callable] = {}
_specs: Dict[Tuple[str, str], dict] = {}

@dataclass
class ActivitySpec:
    task_kind: str
    task_version: str
    inputs: list
    outputs: list


def task(kind: str, version: str, spec: Optional[ActivitySpec] = None):
    def decorator(func: Callable):
        _handlers[(kind, version)] = func
        if spec is not None:
            _specs[(kind, version)] = {
                "inputs": spec.inputs,
                "outputs": spec.outputs,
            }
        return func
    return decorator


def get_handler(kind: str, version: str) -> Optional[Callable]:
    return _handlers.get((kind, version))


def bind_spec_from_yaml(kind: str, version: str, yaml_path: str) -> None:
    with open(yaml_path, "r") as f:
        data = yaml.safe_load(f)
    # Minimal mapping: in repo spec is a whole workflow; caller passes matched step
    _specs[(kind, version)] = data


def get_spec(kind: str, version: str) -> Optional[dict]:
    return _specs.get((kind, version))
