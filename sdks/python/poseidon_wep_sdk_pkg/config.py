from __future__ import annotations
import os
from typing import List, Optional, Union, Any, Dict
from pydantic import BaseModel, Field, validator
import yaml


class TlsConfig(BaseModel):
    enabled: bool = False
    cert_file: str = ""
    key_file: str = ""
    ca_cert_file: str = ""


class ServerConfig(BaseModel):
    host: str = "127.0.0.1"
    port: int = 7070
    tls: TlsConfig = Field(default_factory=TlsConfig)


class RuntimeConfig(BaseModel):
    max_concurrency: Union[int, str] = "auto"
    tags: List[str] = Field(default_factory=list)
    handler_timeout_s: int = 600
    graceful_shutdown_s: int = 15

    @validator("max_concurrency")
    def _validate_mc(cls, v: Union[int, str]) -> Union[int, str]:
        if isinstance(v, int) and v <= 0:
            raise ValueError("max_concurrency must be > 0 or 'auto'")
        if isinstance(v, str) and v != "auto":
            raise ValueError("max_concurrency must be integer or 'auto'")
        return v


class SpecItem(BaseModel):
    kind: str
    version: str
    path: str


class SpecsConfig(BaseModel):
    strict_specs: bool = False
    list: List[SpecItem] = Field(default_factory=list)


class LoggingConfig(BaseModel):
    level: str = "info"
    json: bool = False


class ProtocolConfig(BaseModel):
    proto_min: str = "1.0.0"
    proto_max: str = "1.0.0"


class WepConfig(BaseModel):
    server: ServerConfig = Field(default_factory=ServerConfig)
    runtime: RuntimeConfig = Field(default_factory=RuntimeConfig)
    specs: SpecsConfig = Field(default_factory=SpecsConfig)
    logging: LoggingConfig = Field(default_factory=LoggingConfig)
    protocol: ProtocolConfig = Field(default_factory=ProtocolConfig)


def _set_nested(d: Dict[str, Any], keys: List[str], value: Any) -> None:
    cur = d
    for k in keys[:-1]:
        if k not in cur or not isinstance(cur[k], dict):
            cur[k] = {}
        cur = cur[k]
    cur[keys[-1]] = value


def _apply_env_overrides(cfg: Dict[str, Any], prefix: str = "WEP__") -> None:
    for k, v in os.environ.items():
        if not k.startswith(prefix):
            continue
        path = k[len(prefix):].split("__")
        # Normalize to lower-case keys to match YAML style
        path = [p.lower() for p in path]
        # Basic type coercion for ints/bools
        vv: Any = v
        if v.lower() in ("true", "false"):
            vv = v.lower() == "true"
        else:
            try:
                if "." not in v:
                    vv = int(v)
            except Exception:
                pass
        _set_nested(cfg, path, vv)


def load_wep_config(path: Optional[str]) -> WepConfig:
    data: Dict[str, Any] = {}
    if path and os.path.exists(path):
        with open(path, "r") as f:
            data = yaml.safe_load(f) or {}
    _apply_env_overrides(data, prefix="WEP__")
    return WepConfig.parse_obj(data)


def resolve_max_concurrency(mc: Union[int, str]) -> int:
    if isinstance(mc, int):
        return mc
    import multiprocessing as mp
    return max(1, mp.cpu_count())


