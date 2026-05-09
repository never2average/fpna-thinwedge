#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: check-db-sandbox-readiness.sh [--dry-run] [--include-branch-lifecycle] [--allow-mutation]

Runs the bottom-up readiness probes for ThinWedge finance DB sandboxing. By
default it avoids mutation-gated Ardent branch creation. Use both
--include-branch-lifecycle and --allow-mutation after explicit approval to create
and delete a temporary Ardent branch.

Common env vars:
  THINWEDGE_BILLING_AWS_PROFILE
  THINWEDGE_DB_OPS_AWS_PROFILE
  THINWEDGE_RDS_DB_INSTANCE
  THINWEDGE_ARDENT_CLI
  THINWEDGE_ARDENT_CONNECTOR
EOF
}

dry_run=0
include_branch_lifecycle=0
allow_mutation=0
while [[ $# -gt 0 ]]; do
  case "$1" in
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
"${SCRIPT_DIR}/check-rds-postgres-readiness.sh" "${args[@]}"
"${SCRIPT_DIR}/check-ardent-auth.sh" "${args[@]}"
"${SCRIPT_DIR}/check-ardent-connector.sh" "${args[@]}"
if [[ "${include_branch_lifecycle}" == "1" ]]; then
  "${SCRIPT_DIR}/check-ardent-branch-lifecycle.sh" "${mutation_args[@]}"
else
  echo '[thinwedge-probe] skipping mutation-gated Ardent branch lifecycle probe'
fi
echo '[thinwedge-probe] DB sandbox readiness probe suite passed'
