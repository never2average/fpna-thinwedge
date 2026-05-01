# Getting Started

This is the fastest path from install to a multi-turn thread using the public SDK surface.

The SDK is experimental. Treat the API, bundled runtime strategy, and packaging details as unstable until the first public release.

## 1) Install

From repo root:

```bash
cd sdk/python
uv sync
source .venv/bin/activate
```

Requirements:

- Python `>=3.10`
- uv
- installed `openai-thinwedge-cli-bin` runtime package, or an explicit `thinwedge_bin` override
- local ThinWedge auth/session configured

## 2) Run your first turn (sync)

```python
from thinwedge_app_server import ThinWedge

with ThinWedge() as thinwedge:
    server = thinwedge.metadata.serverInfo
    print("Server:", None if server is None else server.name, None if server is None else server.version)

    thread = thinwedge.thread_start(model="gpt-5.4", config={"model_reasoning_effort": "high"})
    result = thread.run("Say hello in one sentence.")

    print("Thread:", thread.id)
    print("Text:", result.final_response)
    print("Items:", len(result.items))
```

What happened:

- `ThinWedge()` started and initialized `thinwedge app-server`.
- `thread_start(...)` created a thread.
- `thread.run("...")` started a turn, consumed events until completion, and returned the final assistant response plus collected items and usage.
- `result.final_response` is `None` when no final-answer or phase-less assistant message item completes for the turn.
- use `thread.turn(...)` when you need a `TurnHandle` for streaming, steering, interrupting, or turn IDs/status
- one client can have only one active turn consumer (`thread.run(...)`, `TurnHandle.stream()`, or `TurnHandle.run()`) at a time in the current experimental build

## 3) Continue the same thread (multi-turn)

```python
from thinwedge_app_server import ThinWedge

with ThinWedge() as thinwedge:
    thread = thinwedge.thread_start(model="gpt-5.4", config={"model_reasoning_effort": "high"})

    first = thread.run("Summarize Rust ownership in 2 bullets.")
    second = thread.run("Now explain it to a Python developer.")

    print("first:", first.final_response)
    print("second:", second.final_response)
```

## 4) Async parity

Use `async with AsyncThinWedge()` as the normal async entrypoint. `AsyncThinWedge`
initializes lazily, and context entry makes startup/shutdown explicit.

```python
import asyncio
from thinwedge_app_server import AsyncThinWedge


async def main() -> None:
    async with AsyncThinWedge() as thinwedge:
        thread = await thinwedge.thread_start(model="gpt-5.4", config={"model_reasoning_effort": "high"})
        result = await thread.run("Continue where we left off.")
        print(result.final_response)


asyncio.run(main())
```

## 5) Resume an existing thread

```python
from thinwedge_app_server import ThinWedge

THREAD_ID = "thr_123"  # replace with a real id

with ThinWedge() as thinwedge:
    thread = thinwedge.thread_resume(THREAD_ID)
    result = thread.run("Continue where we left off.")
    print(result.final_response)
```

## 6) Generated models

The convenience wrappers live at the package root, but the canonical app-server models live under:

```python
from thinwedge_app_server.generated.v2_all import Turn, TurnStatus, ThreadReadResponse
```

## 7) Next stops

- API surface and signatures: `docs/api-reference.md`
- Common decisions/pitfalls: `docs/faq.md`
- End-to-end runnable examples: `examples/README.md`
