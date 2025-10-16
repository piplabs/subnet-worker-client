from typing import Callable, Dict, Tuple

_handlers: Dict[Tuple[str, str], Callable] = {}

def task(kind: str, version: str):
    def decorator(func: Callable):
        _handlers[(kind, version)] = func
        return func
    return decorator

def get_handler(kind: str, version: str) -> Callable | None:
    return _handlers.get((kind, version))
