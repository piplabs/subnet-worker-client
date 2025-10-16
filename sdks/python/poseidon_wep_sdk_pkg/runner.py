import asyncio
from .server import WepServer

async def run_server(host: str = "127.0.0.1", port: int = 7070, max_concurrency: int = 4, tags: list[str] = None):
    server = WepServer(host=host, port=port, max_concurrency=max_concurrency, tags=tags)
    await server.start()

if __name__ == "__main__":
    asyncio.run(run_server())
