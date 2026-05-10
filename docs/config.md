# Configuration

For basic configuration instructions, see [this documentation](https://developers.thinwedge.com/thinwedge/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.thinwedge.com/thinwedge/config-advanced).

For a full configuration reference, see [this documentation](https://developers.thinwedge.com/thinwedge/config-reference).

## Connecting to MCP servers

ThinWedge can connect to MCP servers configured in `~/.thinwedge/config.toml`. See the configuration reference for the latest MCP server options:

- https://developers.thinwedge.com/thinwedge/config-reference

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
  { type = "plugin", id = "slack@thinwedge-curated" },
  { type = "connector", id = "connector_google_calendar" },
]
```

## Notify

ThinWedge can run a notification hook when the agent finishes a turn. See the configuration reference for the latest notification settings:

- https://developers.thinwedge.com/thinwedge/config-reference

When ThinWedge knows which client started the turn, the legacy notify JSON payload also includes a top-level `client` field. The TUI reports `thinwedge-tui`, and the app server reports the `clientInfo.name` value from `initialize`.

## JSON Schema

The generated JSON Schema for `config.toml` lives at `thinwedge-rs/core/config.schema.json`.

## Finance DB sandboxing

ThinWedge can keep finance/database agents away from production state by first
validating a source provider, then handing agents only disposable sandbox
database state. Neon is the default provider path because it can be checked from
a Postgres URL plus Neon API metadata. Ardent remains optional: use it when you
want an external branch backend after the source provider is proven.

The non-secret configuration shape is:

```toml
[billing]
aws_profile = "fpna-billing"
role_arn = "arn:aws:iam::123456789012:role/fpna-billing"
region = "us-east-1"

[db_ops]
aws_profile = "fpna-db-ops"
role_arn = "arn:aws:iam::123456789012:role/fpna-db-ops"
region = "us-west-2"

[db_sandbox]
enabled = true
provider = "neon"
source_url_env = "THINWEDGE_ARDENT_SOURCE_DATABASE_URL"
neon_api_key_env = "THINWEDGE_NEON_API_KEY"
neon_project_id_env = "THINWEDGE_NEON_PROJECT_ID"
neon_project_id = "twilight-lab-63846303"
branch_backend = "none"

[ardent]
enabled = true
cli_path = "ardent"
default_connector = "fpna-prod"
branch_name_prefix = "thinwedge-agent"
branch_ttl_minutes = 60
data_plane = "byoc"
```

Use `aws_profile` for local workstation setup. Production deployments should
prefer role-based or managed credential providers that resolve to narrowly scoped
AWS credentials. Source database URLs, API keys, and branch URLs are not config
values; they should come from secure stores or environment variables. Agents
should only receive disposable branch or sandbox `DATABASE_URL` values.

Useful CLI entry points:

```bash
thinwedge db-sandbox status
thinwedge db-sandbox configure --enabled --provider neon --neon-project-id <neon-project-id> --branch-backend none --dry-run
thinwedge db-sandbox preflight --dry-run
thinwedge db-sandbox preflight --provider neon
thinwedge ardent status --dry-run
thinwedge ardent login --dry-run
thinwedge ardent configure --enabled --billing-profile fpna-billing --billing-role-arn arn:aws:iam::123456789012:role/fpna-billing --db-ops-profile fpna-db-ops --db-ops-role-arn arn:aws:iam::123456789012:role/fpna-db-ops --connector fpna-prod --data-plane byoc --dry-run --no-prompt
thinwedge ardent connector create --connector fpna-prod --source-url-env THINWEDGE_ARDENT_SOURCE_DATABASE_URL --dry-run
thinwedge ardent connector create --source-provider neon --connector fpna-neon --neon-project-id <neon-project-id> --dry-run
thinwedge ardent branch create --connector fpna-prod --name thinwedge-agent-test --print-env --dry-run
thinwedge ardent branch delete thinwedge-agent-test --connector fpna-prod --dry-run
```

`thinwedge db-sandbox configure` stores only non-secret provider metadata. The
interactive `thinwedge login` flow offers the same setup after the OpenRouter API
token and optional capability prompts. `branch_backend = "none"` is the default
for provider-only validation; switch it to `"ardent"` only after deciding Ardent
should manage disposable database branches.

`thinwedge ardent connector create` is intentionally mutation-gated for live runs
because it attaches a production source database to Ardent. Pass
`--allow-mutation` only after that blast radius is approved. The source URL is
read from the named environment variable and is never printed by ThinWedge. For
Neon BYOC, set `THINWEDGE_NEON_API_KEY` and `THINWEDGE_NEON_PROJECT_ID` or pass
`--neon-project-id`; ThinWedge redacts the API key in dry-runs and error
context.
Choosing `managed` versus `byoc` for `ardent.data_plane` is also an explicit
deployment-boundary decision; record that choice during setup instead of treating
it as an automatic default.

Validate the external contracts bottom-up before trusting the integration. This
is the production-scale workflow for a technical CFO/operator: bring a source DB
URL/API key, run preflight, and only then allow agents to use disposable DB
state. Start with dry-run wiring:

```bash
scripts/probes/check-db-sandbox-readiness.sh --dry-run
```

Then run the non-mutating live readiness suite from a shell with AWS, Ardent,
`nc`, `psql`, and the DB setup role URL configured:

```bash
cp scripts/probes/db-sandbox-readiness.env.example scripts/probes/db-sandbox-readiness.env
# Fill scripts/probes/db-sandbox-readiness.env locally, then load it:
set -a
. scripts/probes/db-sandbox-readiness.env
set +a

scripts/probes/check-db-sandbox-readiness.sh --source-provider neon --branch-backend none
```

Live branch creation/deletion is mutation-gated and requires explicit opt-in.
Use this only when `branch_backend = "ardent"` is part of the deployment:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-db-sandbox-readiness.sh --source-provider neon --branch-backend ardent --include-branch-lifecycle --allow-mutation
```

For Neon, validate the source and Neon API before attempting Ardent BYOC setup:

```bash
THINWEDGE_DB_SOURCE_PROVIDER=neon scripts/probes/check-db-sandbox-readiness.sh
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-neon-postgres-readiness.sh --include-api-branch-smoke
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-ardent-connector.sh --create --source-provider neon
```

If Ardent BYOC Neon returns `snapshot_max_connections` during connector setup,
the local Neon project may still be correctly configured. That error is from
Ardent's BYOC setup path; clean up the failed connector and retry after Ardent
fixes Neon BYOC.

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
