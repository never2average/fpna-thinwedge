#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-neon-postgres-readiness.sh [--api-key-env VAR] [--project-id ID] [--source-url-env VAR] [--include-api-branch-smoke] [--dry-run] [--allow-mutation]

Checks Neon-specific prerequisites before attempting Ardent BYOC Neon setup. It
verifies the Neon API key and project id, then delegates Postgres checks to the
generic source readiness probe. The optional Neon API branch smoke creates and
deletes a temporary Neon branch and is mutation-gated.

Environment:
  THINWEDGE_NEON_API_KEY
  THINWEDGE_NEON_PROJECT_ID
  THINWEDGE_ARDENT_SOURCE_DATABASE_URL
EOF
}

api_key_env="THINWEDGE_NEON_API_KEY"
project_id="${THINWEDGE_NEON_PROJECT_ID:-}"
source_url_env="THINWEDGE_ARDENT_SOURCE_DATABASE_URL"
include_api_branch_smoke=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-key-env) api_key_env="${2:?missing env var}"; shift 2 ;;
    --project-id) project_id="${2:?missing project id}"; shift 2 ;;
    --source-url-env) source_url_env="${2:?missing env var}"; shift 2 ;;
    --include-api-branch-smoke) include_api_branch_smoke=1; shift ;;
    --dry-run) TW_PROBE_DRY_RUN=1; shift ;;
    --verbose) TW_PROBE_VERBOSE=1; shift ;;
    --allow-mutation) TW_PROBE_ALLOW_MUTATION=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) probe_fail "unknown argument: $1" ;;
  esac
done

if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
  probe_info "dry-run: would verify Neon project metadata, endpoints, Postgres source readiness, and optional Neon branch API lifecycle"
  probe_info "Neon Postgres readiness probe passed"
  exit 0
fi

probe_require_cmd python3
api_key="${!api_key_env:-}"
[[ -n "${api_key}" ]] || probe_fail "${api_key_env} must contain a Neon API key"
[[ -n "${project_id}" ]] || probe_fail "THINWEDGE_NEON_PROJECT_ID or --project-id must contain the Neon project id"

probe_info "checking Neon project metadata project_id=${project_id}"
NEON_API_KEY="${api_key}" NEON_PROJECT_ID="${project_id}" python3 - <<'PY'
import json
import os
import sys
import urllib.error
import urllib.request

api_key = os.environ["NEON_API_KEY"]
project_id = os.environ["NEON_PROJECT_ID"]
base = "https://console.neon.tech/api/v2"
headers = {"Authorization": f"Bearer {api_key}", "Accept": "application/json"}

def request(path, method="GET", body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data, method=method, headers=headers | ({"Content-Type": "application/json"} if data else {}))
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return resp.status, json.loads(resp.read().decode() or "{}")
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode(errors="replace")
        raise SystemExit(f"Neon API {method} {path} failed with HTTP {exc.code}: {detail[:300]}")

status, project = request(f"/projects/{project_id}")
if status != 200:
    raise SystemExit(f"Neon project lookup returned HTTP {status}")
project_obj = project.get("project") or {}
settings = project_obj.get("settings") or {}
print(
    "[thinwedge-probe] neon.project="
    + json.dumps(
        {
            "id": project_obj.get("id"),
            "name": project_obj.get("name"),
            "region_id": project_obj.get("region_id"),
            "pg_version": project_obj.get("pg_version"),
            "enable_logical_replication": settings.get("enable_logical_replication"),
        },
        sort_keys=True,
    )
)
if settings.get("enable_logical_replication") is not True:
    raise SystemExit("Neon project must have logical replication enabled")

status, endpoints = request(f"/projects/{project_id}/endpoints")
if status != 200:
    raise SystemExit(f"Neon endpoints lookup returned HTTP {status}")
ready = [
    {
        "id": endpoint.get("id"),
        "type": endpoint.get("type"),
        "state": endpoint.get("current_state"),
        "min_cu": endpoint.get("autoscaling_limit_min_cu"),
        "max_cu": endpoint.get("autoscaling_limit_max_cu"),
    }
    for endpoint in endpoints.get("endpoints", [])
    if endpoint.get("type") == "read_write" and endpoint.get("disabled") is False
]
print("[thinwedge-probe] neon.read_write_endpoints=" + json.dumps(ready, sort_keys=True))
if not ready:
    raise SystemExit("Neon project must have an enabled read_write endpoint")
PY

postgres_args=(--source-url-env "${source_url_env}")
[[ "${TW_PROBE_ALLOW_MUTATION}" == "1" ]] && postgres_args+=(--allow-mutation)
"${SCRIPT_DIR}/check-postgres-source-readiness.sh" "${postgres_args[@]}"

if [[ "${include_api_branch_smoke}" == "1" ]]; then
  probe_require_mutation_allowed "creating and deleting a temporary Neon API branch"
  probe_info "checking Neon API branch create/delete lifecycle"
  NEON_API_KEY="${api_key}" NEON_PROJECT_ID="${project_id}" python3 - <<'PY'
import json
import os
import time
import urllib.error
import urllib.request

api_key = os.environ["NEON_API_KEY"]
project_id = os.environ["NEON_PROJECT_ID"]
base = "https://console.neon.tech/api/v2"
headers = {"Authorization": f"Bearer {api_key}", "Accept": "application/json", "Content-Type": "application/json"}

def request(path, method="GET", body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return resp.status, json.loads(resp.read().decode() or "{}")
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode(errors="replace")
        raise SystemExit(f"Neon API {method} {path} failed with HTTP {exc.code}: {detail[:300]}")

def wait_operations(operations):
    for operation in operations or []:
        operation_id = operation.get("id")
        if not operation_id:
            raise SystemExit("Neon operation response did not include an operation id")
        for _ in range(60):
            _, payload = request(f"/projects/{project_id}/operations/{operation_id}")
            status = (payload.get("operation") or {}).get("status")
            print(f"[thinwedge-probe] neon.operation={operation_id} status={status}")
            if status == "finished":
                break
            if status in {"failed", "error"}:
                raise SystemExit(f"Neon operation {operation_id} failed with status={status}")
            time.sleep(2)
        else:
            raise SystemExit(f"Neon operation {operation_id} did not finish within 120 seconds")

branch_name = "thinwedge-api-smoke-" + str(int(time.time()))
_, created = request(f"/projects/{project_id}/branches", "POST", {"branch": {"name": branch_name}})
branch = created.get("branch") or {}
branch_id = branch.get("id")
print(f"[thinwedge-probe] neon.created_branch={branch_id}")
wait_operations(created.get("operations"))
if branch_id:
    _, deleted = request(f"/projects/{project_id}/branches/{branch_id}", "DELETE")
    print(f"[thinwedge-probe] neon.deleted_branch={branch_id}")
    wait_operations(deleted.get("operations"))
PY
else
  probe_warn "skipping Neon API branch lifecycle smoke; rerun with --include-api-branch-smoke --allow-mutation for production sign-off"
fi

probe_info "Neon Postgres readiness probe passed"
