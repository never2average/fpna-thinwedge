# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/thinwedge/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/thinwedge/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/thinwedge/config-reference).

## Connecting to MCP servers

ThinWedge can connect to MCP servers configured in `~/.thinwedge/config.toml`. See the configuration reference for the latest MCP server options:

- https://developers.openai.com/thinwedge/config-reference

MCP tools default to serialized calls. To mark every tool exposed by one server
as eligible for parallel tool calls, set `supports_parallel_tool_calls` on that
server:

```toml
[mcp_servers.docs]
command = "docs-server"
supports_parallel_tool_calls = true
```

Only enable parallel calls for MCP servers whose tools are safe to run at the
same time. If tools read and write shared state, files, databases, or external
resources, review those read/write race conditions before enabling this setting.

## MCP tool approvals

ThinWedge stores approval defaults and per-tool overrides for custom MCP servers
under `mcp_servers` in `~/.thinwedge/config.toml`. Set
`default_tools_approval_mode` on the server to apply a default to every tool,
and use per-tool `approval_mode` entries for exceptions:

```toml
[mcp_servers.docs]
command = "docs-server"
default_tools_approval_mode = "approve"

[mcp_servers.docs.tools.search]
approval_mode = "prompt"
```

## Apps (Connectors)

Use `$` in the composer to insert a ChatGPT connector; the popover lists accessible
apps. The `/apps` command lists available and installed apps. Connected apps appear first
and are labeled as connected; others are marked as can be installed.

ThinWedge stores "never show again" choices for tool suggestions in `config.toml`:

```toml
[tool_suggest]
disabled_tools = [
  { type = "plugin", id = "slack@openai-curated" },
  { type = "connector", id = "connector_google_calendar" },
]
```

## Notify

ThinWedge can run a notification hook when the agent finishes a turn. See the configuration reference for the latest notification settings:

- https://developers.openai.com/thinwedge/config-reference

When ThinWedge knows which client started the turn, the legacy notify JSON payload also includes a top-level `client` field. The TUI reports `thinwedge-tui`, and the app server reports the `clientInfo.name` value from `initialize`.

## JSON Schema

The generated JSON Schema for `config.toml` lives at `thinwedge-rs/core/config.schema.json`.

## SQLite State DB

ThinWedge stores the SQLite-backed state DB under `sqlite_home` (config key) or the
`THINWEDGE_SQLITE_HOME` environment variable. When unset, WorkspaceWrite sandbox
sessions default to a temp directory; other modes default to `THINWEDGE_HOME`.

## Custom CA Certificates

ThinWedge can trust a custom root CA bundle for outbound HTTPS and secure websocket
connections when enterprise proxies or gateways intercept TLS. This applies to
login flows and to ThinWedge's other external connections, including ThinWedge
components that build reqwest clients or secure websocket clients through the
shared `thinwedge-client` CA-loading path and remote MCP connections that use it.

Set `THINWEDGE_CA_CERTIFICATE` to the path of a PEM file containing one or more
certificate blocks to use a ThinWedge-specific CA bundle. If
`THINWEDGE_CA_CERTIFICATE` is unset, ThinWedge falls back to `SSL_CERT_FILE`. If
neither variable is set, ThinWedge uses the system root certificates.

`THINWEDGE_CA_CERTIFICATE` takes precedence over `SSL_CERT_FILE`. Empty values are
treated as unset.

The PEM file may contain multiple certificates. ThinWedge also tolerates OpenSSL
`TRUSTED CERTIFICATE` labels and ignores well-formed `X509 CRL` sections in the
same bundle. If the file is empty, unreadable, or malformed, the affected ThinWedge
HTTP or secure websocket connection reports a user-facing error that points
back to these environment variables.

## Notices

ThinWedge stores "do not show again" flags for some UI prompts under the `[notice]` table.

## Plan mode defaults

`plan_mode_reasoning_effort` lets you set a Plan-mode-specific default reasoning
effort override. When unset, Plan mode uses the built-in Plan preset default
(currently `medium`). When explicitly set (including `none`), it overrides the
Plan preset. The string value `none` means "no reasoning" (an explicit Plan
override), not "inherit the global default". There is currently no separate
config value for "follow the global default in Plan mode".

## Realtime start instructions

`experimental_realtime_start_instructions` lets you replace the built-in
developer message ThinWedge inserts when realtime becomes active. It only affects
the realtime start message in prompt history and does not change websocket
backend prompt settings or the realtime end/inactive message.

Ctrl+C/Ctrl+D quitting uses a ~1 second double-press hint (`ctrl + c again to quit`).
