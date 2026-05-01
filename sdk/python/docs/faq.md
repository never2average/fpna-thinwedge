# FAQ

## Thread vs turn

- A `Thread` is conversation state.
- A `Turn` is one model execution inside that thread.
- Multi-turn chat means multiple turns on the same `Thread`.

## `run()` vs `stream()`

- `TurnHandle.run()` / `AsyncTurnHandle.run()` is the easiest path. It consumes events until completion and returns the canonical generated app-server `Turn` model.
- `TurnHandle.stream()` / `AsyncTurnHandle.stream()` yields raw notifications (`Notification`) so you can react event-by-event.

Choose `run()` for most apps. Choose `stream()` for progress UIs, custom timeout logic, or custom parsing.

## Sync vs async clients

- `ThinWedge` is the sync public API.
- `AsyncThinWedge` is an async replica of the same public API shape.
- Prefer `async with AsyncThinWedge()` for async code. It is the standard path for
  explicit startup/shutdown, and `AsyncThinWedge` initializes lazily on context
  entry or first awaited API use.

If your app is not already async, stay with `ThinWedge`.

## Public kwargs are snake_case

Public API keyword names are snake_case. The SDK still maps them to wire camelCase under the hood.

If you are migrating older code, update these names:

- `approvalPolicy` -> `approval_policy`
- `baseInstructions` -> `base_instructions`
- `developerInstructions` -> `developer_instructions`
- `modelProvider` -> `model_provider`
- `modelProviders` -> `model_providers`
- `sortKey` -> `sort_key`
- `sourceKinds` -> `source_kinds`
- `outputSchema` -> `output_schema`
- `sandboxPolicy` -> `sandbox_policy`

## Why only `thread_start(...)` and `thread_resume(...)`?

The public API keeps only explicit lifecycle calls:

- `thread_start(...)` to create new threads
- `thread_resume(thread_id, ...)` to continue existing threads

This avoids duplicate ways to do the same operation and keeps behavior explicit.

## Why does constructor fail?

`ThinWedge()` is eager: it starts transport and calls `initialize` in `__init__`.

Common causes:

- published runtime package (`thinwedge-thinwedge-cli-bin`) is not installed
- local `thinwedge_bin` override points to a missing file
- local auth/session is missing
- incompatible/old app-server

Maintainers stage releases by building the SDK once and the runtime once per
platform with the same pinned runtime version. Publish `thinwedge-thinwedge-cli-bin`
as platform wheels only; do not publish an sdist:

```bash
cd sdk/python
python scripts/update_sdk_artifacts.py generate-types
python scripts/update_sdk_artifacts.py \
  stage-sdk \
  /tmp/thinwedge-python-release/thinwedge-thinwedge-app-server-sdk \
  --thinwedge-version <thinwedge-release-tag-or-pep440-version>
python scripts/update_sdk_artifacts.py \
  stage-runtime \
  /tmp/thinwedge-python-release/thinwedge-thinwedge-cli-bin \
  /path/to/thinwedge \
  --thinwedge-version <thinwedge-release-tag-or-pep440-version>
```

If you are packaging a binary for a different target than the Python build
host, pass `--platform-tag ...` to `stage-runtime`. The intended one-off matrix
is `macosx_11_0_arm64`, `macosx_10_9_x86_64`, `musllinux_1_1_aarch64`,
`musllinux_1_1_x86_64`, `win_arm64`, and `win_amd64`.

## Why does a turn "hang"?

A turn is complete only when `turn/completed` arrives for that turn ID.

- `run()` waits for this automatically.
- With `stream()`, keep consuming notifications until completion.

## How do I retry safely?

Use `retry_on_overload(...)` for transient overload failures (`ServerBusyError`).

Do not blindly retry all errors. For `InvalidParamsError` or `MethodNotFoundError`, fix inputs/version compatibility instead.

## Common pitfalls

- Starting a new thread for every prompt when you wanted continuity.
- Forgetting to `close()` (or not using context managers).
- Assuming `run()` returns extra SDK-only fields instead of the generated `Turn` model.
- Mixing SDK input classes with raw dicts incorrectly.
