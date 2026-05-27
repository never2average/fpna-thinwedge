#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-postgres-source-readiness.sh [--source-url-env VAR] [--skip-network-check] [--skip-ddl-probe] [--dry-run] [--allow-mutation]

Checks a Postgres source database for Ardent-style branching prerequisites
without printing the source URL. The DDL event-trigger probe creates and drops
temporary objects, so it is mutation-gated behind --allow-mutation.

Environment:
  THINWEDGE_ARDENT_SOURCE_DATABASE_URL
  THINWEDGE_SKIP_DB_NETWORK_CHECK=1
  THINWEDGE_SKIP_DB_DDL_PROBE=1
EOF
}

source_url_env="THINWEDGE_ARDENT_SOURCE_DATABASE_URL"
skip_network_check="${THINWEDGE_SKIP_DB_NETWORK_CHECK:-0}"
skip_ddl_probe="${THINWEDGE_SKIP_DB_DDL_PROBE:-0}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-url-env) source_url_env="${2:?missing env var}"; shift 2 ;;
    --skip-network-check) skip_network_check=1; shift ;;
    --skip-ddl-probe) skip_ddl_probe=1; shift ;;
    --dry-run) TW_PROBE_DRY_RUN=1; shift ;;
    --verbose) TW_PROBE_VERBOSE=1; shift ;;
    --allow-mutation) TW_PROBE_ALLOW_MUTATION=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) probe_fail "unknown argument: $1" ;;
  esac
done

if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
  probe_info "dry-run: would parse redacted Postgres URL, check TCP reachability, verify logical replication privileges, and optionally create/drop a temporary event trigger"
  probe_info "Postgres source readiness probe passed"
  exit 0
fi

probe_require_cmd python3
probe_require_cmd psql
source_url="${!source_url_env:-}"
[[ -n "${source_url}" ]] || probe_fail "${source_url_env} must contain the source database URL"

endpoint="$(
  SOURCE_URL="${source_url}" python3 - <<'PY'
import os
from urllib.parse import urlparse

parsed = urlparse(os.environ["SOURCE_URL"])
host = parsed.hostname or ""
port = parsed.port or 5432
database = (parsed.path or "/").lstrip("/") or "postgres"
if not host:
    raise SystemExit("missing host in source URL")
print(f"{host}\t{port}\t{database}")
PY
)"
host="$(printf '%s' "${endpoint}" | awk -F '\t' '{print $1}')"
port="$(printf '%s' "${endpoint}" | awk -F '\t' '{print $2}')"
database="$(printf '%s' "${endpoint}" | awk -F '\t' '{print $3}')"
probe_info "checking Postgres source readiness host=${host} port=${port} database=${database}"

if [[ "${skip_network_check}" == "1" ]]; then
  probe_warn "skipping TCP reachability check by explicit request"
else
  probe_require_cmd nc
  nc -vz -w 5 "${host}" "${port}" >/dev/null || probe_fail "cannot reach ${host}:${port} from this host"
fi

role_status="$(
  psql "${source_url}" -v ON_ERROR_STOP=1 -Atqc \
    "select current_database() || ' current_user=' || current_user || ' wal_level=' || current_setting(\$\$wal_level\$\$) || ' rolsuper=' || rolsuper || ' rolreplication=' || rolreplication || ' can_create_public=' || has_schema_privilege(current_user,\$\$public\$\$,\$\$CREATE\$\$) from pg_roles where rolname = current_user;"
)"
printf '%s\n' "${role_status}"
[[ "${role_status}" == *" wal_level=logical "* ]] || probe_fail "Postgres source must have wal_level=logical"
[[ "${role_status}" == *" rolreplication=t "* || "${role_status}" == *" rolsuper=t "* ]] || probe_fail "Postgres source role must have rolreplication=true or rolsuper=true"
[[ "${role_status}" == *" can_create_public=t"* ]] || probe_fail "Postgres source role must be able to create objects in schema public"

if [[ "${skip_ddl_probe}" == "1" ]]; then
  probe_warn "skipping DDL event-trigger probe by explicit request"
elif [[ "${TW_PROBE_ALLOW_MUTATION}" == "1" ]]; then
  probe_info "checking temporary event-trigger create/drop capability"
  psql "${source_url}" -v ON_ERROR_STOP=1 -qc \
    "create or replace function thinwedge_probe_evt_fn() returns event_trigger language plpgsql as \$\$ begin perform 1; end; \$\$; create event trigger thinwedge_probe_evt on ddl_command_end execute function thinwedge_probe_evt_fn(); drop event trigger thinwedge_probe_evt; drop function thinwedge_probe_evt_fn();"
else
  probe_warn "skipping temporary event-trigger create/drop probe; rerun with --allow-mutation for production sign-off"
fi

probe_info "Postgres source readiness probe passed"
