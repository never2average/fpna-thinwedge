#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: check-db-sandbox-readiness.sh [--source-provider rds|postgresql|supabase|neon] [--dry-run] [--include-branch-lifecycle] [--allow-mutation]

Runs the bottom-up readiness probes for ThinWedge finance DB sandboxing. By
default it avoids mutation-gated Ardent branch creation. Use both
--include-branch-lifecycle and --allow-mutation after explicit approval to create
and delete a temporary Ardent branch.

Common env vars:
  THINWEDGE_DB_SOURCE_PROVIDER
  THINWEDGE_BILLING_AWS_PROFILE
  THINWEDGE_DB_OPS_AWS_PROFILE
  THINWEDGE_RDS_DB_INSTANCE
  THINWEDGE_ARDENT_SOURCE_DATABASE_URL
  THINWEDGE_NEON_API_KEY
  THINWEDGE_NEON_PROJECT_ID
  THINWEDGE_ARDENT_CLI
  THINWEDGE_ARDENT_CONNECTOR
EOF
}

dry_run=0
include_branch_lifecycle=0
allow_mutation=0
source_provider="${THINWEDGE_DB_SOURCE_PROVIDER:-rds}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-provider) source_provider="${2:?missing source provider}"; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    --include-branch-lifecycle) include_branch_lifecycle=1; shift ;;
    --allow-mutation) allow_mutation=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
done

args=()
[[ "${dry_run}" == "1" ]] && args+=(--dry-run)
mutation_args=()
[[ "${dry_run}" == "1" ]] && mutation_args+=(--dry-run)
[[ "${allow_mutation}" == "1" ]] && mutation_args+=(--allow-mutation)

"${SCRIPT_DIR}/check-aws-billing.sh" "${args[@]}"
"${SCRIPT_DIR}/check-aws-db-ops.sh" "${args[@]}"
case "${source_provider}" in
  rds)
    "${SCRIPT_DIR}/check-rds-postgres-readiness.sh" "${args[@]}"
    ;;
  postgresql|postgres|supabase|self-hosted)
    "${SCRIPT_DIR}/check-postgres-source-readiness.sh" "${mutation_args[@]}"
    ;;
  neon)
    "${SCRIPT_DIR}/check-neon-postgres-readiness.sh" "${mutation_args[@]}"
    ;;
  none|skip)
    echo '[thinwedge-probe] skipping source database readiness probe by explicit provider'
    ;;
  *)
    echo "unknown source provider: ${source_provider}" >&2
    exit 1
    ;;
esac
"${SCRIPT_DIR}/check-ardent-auth.sh" "${args[@]}"
"${SCRIPT_DIR}/check-ardent-connector.sh" "${args[@]}"
if [[ "${include_branch_lifecycle}" == "1" ]]; then
  "${SCRIPT_DIR}/check-ardent-branch-lifecycle.sh" "${mutation_args[@]}"
else
  echo '[thinwedge-probe] skipping mutation-gated Ardent branch lifecycle probe'
fi
echo '[thinwedge-probe] DB sandbox readiness probe suite passed'
