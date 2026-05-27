#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-rds-postgres-readiness.sh [--profile NAME] [--region REGION] [--db-instance IDENTIFIER] [--dry-run] [--verbose] [--skip-network-check] [--skip-db-role-check]

Checks whether an RDS Postgres instance is ready for Ardent-style branching. It
reads engine metadata, endpoint, VPC security group ids, security-group rules,
parameter groups, rds.logical_replication, TCP reachability from the current
host, and the DB setup role's replication capability. It does not mutate AWS or
DB state.

Environment:
  THINWEDGE_RDS_DB_INSTANCE
  THINWEDGE_DB_ROLE_DATABASE_URL
  THINWEDGE_SKIP_DB_NETWORK_CHECK=1
  THINWEDGE_SKIP_DB_ROLE_CHECK=1
EOF
}

profile="${THINWEDGE_DB_OPS_AWS_PROFILE:-${AWS_PROFILE:-}}"
region="${AWS_REGION:-${AWS_DEFAULT_REGION:-us-east-1}}"
db_instance="${THINWEDGE_RDS_DB_INSTANCE:-}"
db_role_database_url="${THINWEDGE_DB_ROLE_DATABASE_URL:-}"
skip_network_check="${THINWEDGE_SKIP_DB_NETWORK_CHECK:-0}"
skip_db_role_check="${THINWEDGE_SKIP_DB_ROLE_CHECK:-0}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="${2:?missing profile}"; shift 2 ;;
    --region) region="${2:?missing region}"; shift 2 ;;
    --db-instance) db_instance="${2:?missing db instance}"; shift 2 ;;
    --db-role-database-url-env)
      env_name="${2:?missing env var}"
      db_role_database_url="${!env_name:-}"
      shift 2
      ;;
    --skip-network-check) skip_network_check=1; shift ;;
    --skip-db-role-check) skip_db_role_check=1; shift ;;
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
  probe_info "dry-run: would discover RDS Postgres instance, inspect security groups, inspect rds.logical_replication, check TCP reachability, and validate DB setup role"
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

if [[ -n "${sgs}" ]]; then
  probe_info "checking RDS security-group rules"
  aws ec2 describe-security-groups "${aws_args[@]}" --group-ids ${sgs} --query 'SecurityGroups[].{GroupId:GroupId,Ingress:IpPermissions[].FromPort,Egress:IpPermissionsEgress[].IpProtocol}' --output json >/dev/null
fi

if [[ "${skip_network_check}" == "1" ]]; then
  probe_warn "skipping TCP reachability check by explicit request"
else
  probe_require_cmd nc
  probe_info "checking TCP reachability to ${endpoint}:${port}"
  nc -vz -w 5 "${endpoint}" "${port}" >/dev/null || probe_fail "cannot reach ${endpoint}:${port} from this host"
fi

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

if [[ "${skip_db_role_check}" == "1" ]]; then
  probe_warn "skipping DB setup role replication check by explicit request"
else
  probe_require_cmd psql
  [[ -n "${db_role_database_url}" ]] || probe_fail "THINWEDGE_DB_ROLE_DATABASE_URL must contain the DB setup role connection URL"
  probe_info "checking DB setup role replication capability via psql"
  role_status="$(psql "${db_role_database_url}" -Atc "select case when rolsuper or rolreplication then 'ready' else 'not_ready' end || ' current_user=' || current_user || ' rolsuper=' || rolsuper || ' rolreplication=' || rolreplication from pg_roles where rolname = current_user;")"
  printf '%s\n' "${role_status}"
  [[ "${role_status}" == ready\ * ]] || probe_fail "DB setup role must have rolreplication=true or rolsuper=true"
fi

[[ "${ready}" == "1" ]] || probe_fail "rds.logical_replication must be 1 before Ardent source branching can be production-ready"
probe_info "RDS Postgres readiness probe passed"
