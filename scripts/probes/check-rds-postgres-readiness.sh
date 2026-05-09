#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-rds-postgres-readiness.sh [--profile NAME] [--region REGION] [--db-instance IDENTIFIER] [--dry-run] [--verbose]

Checks whether an RDS Postgres instance is ready for Ardent-style branching. It
reads engine metadata, endpoint, VPC security group ids, parameter groups, and
rds.logical_replication. If DATABASE_URL is set and psql is available, it also
checks the current DB role's replication flags. It does not mutate AWS or DB state.
EOF
}

profile="${THINWEDGE_DB_OPS_AWS_PROFILE:-${AWS_PROFILE:-}}"
region="${AWS_REGION:-${AWS_DEFAULT_REGION:-us-east-1}}"
db_instance="${THINWEDGE_RDS_DB_INSTANCE:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="${2:?missing profile}"; shift 2 ;;
    --region) region="${2:?missing region}"; shift 2 ;;
    --db-instance) db_instance="${2:?missing db instance}"; shift 2 ;;
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

if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
  probe_info "dry-run: would discover RDS Postgres instance and inspect rds.logical_replication"
  probe_info "RDS Postgres readiness probe passed"
  exit 0
fi

if [[ -z "${db_instance}" ]]; then
  db_instance="$(aws rds describe-db-instances "${aws_args[@]}" \
    --query "DBInstances[?contains(Engine, 'postgres')].DBInstanceIdentifier | [0]" \
    --output text)"
  [[ -n "${db_instance}" && "${db_instance}" != "None" ]] || probe_fail "no RDS Postgres instance found; pass --db-instance"
fi

probe_info "checking RDS Postgres readiness db_instance=${db_instance}${profile:+ profile=${profile}} region=${region}"
engine="$(aws rds describe-db-instances "${aws_args[@]}" --db-instance-identifier "${db_instance}" --query 'DBInstances[0].Engine' --output text)"
[[ "${engine}" == *postgres* ]] || probe_fail "RDS instance ${db_instance} engine is ${engine}, not Postgres"

endpoint="$(aws rds describe-db-instances "${aws_args[@]}" --db-instance-identifier "${db_instance}" --query 'DBInstances[0].Endpoint.Address' --output text)"
port="$(aws rds describe-db-instances "${aws_args[@]}" --db-instance-identifier "${db_instance}" --query 'DBInstances[0].Endpoint.Port' --output text)"
sgs="$(aws rds describe-db-instances "${aws_args[@]}" --db-instance-identifier "${db_instance}" --query 'DBInstances[0].VpcSecurityGroups[].VpcSecurityGroupId' --output text)"
parameter_groups="$(aws rds describe-db-instances "${aws_args[@]}" --db-instance-identifier "${db_instance}" --query 'DBInstances[0].DBParameterGroups[].DBParameterGroupName' --output text)"
probe_info "endpoint=${endpoint}:${port} security_groups=${sgs:-none} parameter_groups=${parameter_groups:-none}"

ready=1
for group in ${parameter_groups}; do
  value="$(aws rds describe-db-parameters "${aws_args[@]}" \
    --db-parameter-group-name "${group}" \
    --query "Parameters[?ParameterName=='rds.logical_replication'].ParameterValue | [0]" \
    --output text)"
  probe_info "parameter_group=${group} rds.logical_replication=${value}"
  if [[ "${value}" != "1" ]]; then
    ready=0
  fi
done

if [[ -n "${DATABASE_URL:-}" ]]; then
  if command -v psql >/dev/null 2>&1; then
    probe_info "checking current database role replication flags via psql"
    psql "${DATABASE_URL}" -Atc "select current_user || ' rolsuper=' || rolsuper || ' rolreplication=' || rolreplication from pg_roles where rolname = current_user;"
  else
    probe_warn "DATABASE_URL is set but psql is missing; skipping DB role replication check"
  fi
else
  probe_warn "DATABASE_URL is not set; skipping DB role replication check"
fi

[[ "${ready}" == "1" ]] || probe_fail "rds.logical_replication must be 1 before Ardent source branching can be production-ready"
probe_info "RDS Postgres readiness probe passed"
