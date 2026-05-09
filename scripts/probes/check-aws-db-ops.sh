#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-aws-db-ops.sh [--profile NAME] [--region REGION] [--dry-run] [--verbose]

Validates the DB Ops AWS identity. The live check verifies STS, RDS read access,
Secrets Manager read/list access, and SSM Parameter Store read/list access. It
does not mutate AWS resources.

If THINWEDGE_DB_SECRET_ID or THINWEDGE_DB_SSM_PARAMETER are set, the probe reads
that specific DB connection secret/parameter. Otherwise it verifies list access.

Environment fallbacks:
  THINWEDGE_DB_OPS_AWS_PROFILE
  THINWEDGE_DB_SECRET_ID
  THINWEDGE_DB_SSM_PARAMETER
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
if [[ -n "${THINWEDGE_DB_SECRET_ID:-}" ]]; then
  probe_maybe_dry_run aws secretsmanager get-secret-value "${aws_args[@]}" --secret-id "${THINWEDGE_DB_SECRET_ID}" --query ARN --output text >/dev/null
else
  probe_maybe_dry_run aws secretsmanager list-secrets "${aws_args[@]}" --max-results 5 --output json >/dev/null
fi
if [[ -n "${THINWEDGE_DB_SSM_PARAMETER:-}" ]]; then
  probe_maybe_dry_run aws ssm get-parameter "${aws_args[@]}" --name "${THINWEDGE_DB_SSM_PARAMETER}" --with-decryption --query Parameter.ARN --output text >/dev/null
else
  probe_maybe_dry_run aws ssm describe-parameters "${aws_args[@]}" --max-items 5 --output json >/dev/null
fi
probe_info "AWS DB Ops probe passed"
