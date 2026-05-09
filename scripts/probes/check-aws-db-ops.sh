#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-aws-db-ops.sh [--profile NAME] [--region REGION] [--dry-run] [--verbose]

Validates the DB Ops AWS identity. The live check verifies STS, RDS read access,
and Secrets Manager read/list access. It does not mutate AWS resources.

Environment fallbacks:
  THINWEDGE_DB_OPS_AWS_PROFILE
  AWS_PROFILE
  AWS_REGION / AWS_DEFAULT_REGION
EOF
}

profile="${THINWEDGE_DB_OPS_AWS_PROFILE:-${AWS_PROFILE:-}}"
region="${AWS_REGION:-${AWS_DEFAULT_REGION:-us-east-1}}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="${2:?missing profile}"; shift 2 ;;
    --region) region="${2:?missing region}"; shift 2 ;;
    --dry-run) TW_PROBE_DRY_RUN=1; shift ;;
    --verbose) TW_PROBE_VERBOSE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) probe_fail "unknown argument: $1" ;;
  esac
done

[[ "${TW_PROBE_DRY_RUN}" == "1" ]] || probe_require_cmd aws
aws_args=()
[[ -n "${profile}" ]] && aws_args+=(--profile "${profile}")
[[ -n "${region}" ]] && aws_args+=(--region "${region}")

probe_info "checking AWS DB Ops identity${profile:+ profile=${profile}} region=${region}"
probe_maybe_dry_run aws sts get-caller-identity "${aws_args[@]}" --output json >/dev/null
probe_maybe_dry_run aws rds describe-db-instances "${aws_args[@]}" --max-items 20 --output json >/dev/null
probe_maybe_dry_run aws secretsmanager list-secrets "${aws_args[@]}" --max-results 5 --output json >/dev/null
probe_info "AWS DB Ops probe passed"
