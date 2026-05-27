#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-aws-billing.sh [--profile NAME] [--region REGION] [--dry-run] [--verbose]

Validates the billing AWS identity for finance agents. The live check verifies
STS identity, Cost Explorer, CUR, Budgets, and account metadata read access. It
does not mutate AWS resources.

Environment fallbacks:
  THINWEDGE_BILLING_AWS_PROFILE
  AWS_PROFILE
  AWS_REGION / AWS_DEFAULT_REGION
EOF
}

profile="${THINWEDGE_BILLING_AWS_PROFILE:-${AWS_PROFILE:-}}"
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

portable_utc_days_ago() {
  local days="$1"
  if date -u -d "${days} days ago" +%Y-%m-%d >/dev/null 2>&1; then
    date -u -d "${days} days ago" +%Y-%m-%d
  else
    date -u -v-"${days}"d +%Y-%m-%d
  fi
}

probe_info "checking AWS billing identity${profile:+ profile=${profile}} region=${region}"
if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
  probe_maybe_dry_run aws sts get-caller-identity "${aws_args[@]}" --output json >/dev/null
  account_id="000000000000"
else
  account_id="$(aws sts get-caller-identity "${aws_args[@]}" --query Account --output text)"
fi

start_date="$(portable_utc_days_ago 3)"
end_date="$(portable_utc_days_ago 2)"
probe_info "checking Cost Explorer read access"
probe_maybe_dry_run aws ce get-cost-and-usage \
  "${aws_args[@]}" \
  --time-period "Start=${start_date},End=${end_date}" \
  --granularity DAILY \
  --metrics UnblendedCost \
  --output json >/dev/null
probe_info "checking CUR read access"
probe_maybe_dry_run aws cur describe-report-definitions "${aws_args[@]}" --output json >/dev/null
probe_info "checking Budgets read access"
probe_maybe_dry_run aws budgets describe-budgets \
  "${aws_args[@]}" \
  --account-id "${account_id}" \
  --max-results 5 \
  --output json >/dev/null
probe_info "checking account metadata read access"
probe_maybe_dry_run aws iam get-account-summary "${aws_args[@]}" --output json >/dev/null
probe_info "AWS billing probe passed"
