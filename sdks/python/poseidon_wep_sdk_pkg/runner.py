import asyncio
from typing import Optional, List
from .server import WepServer
from .config import load_wep_config, resolve_max_concurrency
from .registry import bind_spec_from_yaml

async def run_server(host: str = "127.0.0.1", port: int = 7070, max_concurrency: int = 4, tags: Optional[List[str]] = None, config_path: Optional[str] = None):
    if config_path:
        cfg = load_wep_config(config_path)
        # Bind specs listed in config
        for s in cfg.specs.list:
            bind_spec_from_yaml(s.kind, s.version, s.path)
        if cfg.specs.strict_specs:
            # TODO: add optional strict handler check
            pass
        mc = resolve_max_concurrency(cfg.runtime.max_concurrency)
        server = WepServer(host=cfg.server.host, port=cfg.server.port, max_concurrency=mc, tags=cfg.runtime.tags, proto_min=cfg.protocol.proto_min, proto_max=cfg.protocol.proto_max)
        await server.start()
    else:
        server = WepServer(host=host, port=port, max_concurrency=max_concurrency, tags=tags)
        await server.start()

if __name__ == "__main__":
    asyncio.run(run_server())
