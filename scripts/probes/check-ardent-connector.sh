#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-ardent-connector.sh [--ardent PATH] [--connector NAME] [--create] [--source-provider postgresql|neon] [--connection-string-env VAR] [--neon-api-key-env VAR] [--neon-project-id ID] [--dry-run] [--allow-mutation]

Checks Ardent connector readiness without exposing the source DB URL. By default
it lists connectors, fails if no connectors exist, and optionally verifies that
NAME appears in the listing.
Connector creation is mutation-gated and reads secrets only from the named
environment variables. Neon uses Ardent's BYOC command and never prints the API
key.
EOF
}

ardent="${THINWEDGE_ARDENT_CLI:-ardent}"
connector="${THINWEDGE_ARDENT_CONNECTOR:-}"
create=0
source_provider="${THINWEDGE_DB_SOURCE_PROVIDER:-postgresql}"
connection_string_env="THINWEDGE_ARDENT_SOURCE_DATABASE_URL"
neon_api_key_env="THINWEDGE_NEON_API_KEY"
neon_project_id="${THINWEDGE_NEON_PROJECT_ID:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ardent) ardent="${2:?missing ardent path}"; shift 2 ;;
    --connector) connector="${2:?missing connector}"; shift 2 ;;
    --create) create=1; shift ;;
    --source-provider) source_provider="${2:?missing source provider}"; shift 2 ;;
    --connection-string-env) connection_string_env="${2:?missing env var}"; shift 2 ;;
    --neon-api-key-env) neon_api_key_env="${2:?missing env var}"; shift 2 ;;
    --neon-project-id) neon_project_id="${2:?missing Neon project id}"; shift 2 ;;
    --dry-run) TW_PROBE_DRY_RUN=1; shift ;;
    --allow-mutation) TW_PROBE_ALLOW_MUTATION=1; shift ;;
    --verbose) TW_PROBE_VERBOSE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) probe_fail "unknown argument: $1" ;;
  esac
done

if [[ "${TW_PROBE_DRY_RUN}" != "1" ]]; then
  if [[ "${ardent}" == */* ]]; then
    [[ -x "${ardent}" ]] || probe_fail "Ardent CLI is not executable: ${ardent}"
  else
    probe_require_cmd "${ardent}"
  fi
fi

if [[ "${create}" == "1" ]]; then
  create_args=(connector create)
  [[ -n "${connector}" ]] && create_args+=(--name "${connector}")
  case "${source_provider}" in
    postgresql|postgres|rds|supabase|self-hosted)
      create_args+=(postgresql)
      redacted_tail="<redacted-source-url>"
      ;;
    neon)
      create_args+=(--byoc neon --api-key "<redacted-neon-api-key>" --project-id "${neon_project_id:-<neon-project-id>}" postgresql)
      redacted_tail=""
      ;;
    *)
      probe_fail "unsupported Ardent source provider: ${source_provider}"
      ;;
  esac
  if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
    probe_info "dry-run: ${ardent} ${create_args[*]}${redacted_tail:+ ${redacted_tail}}"
    probe_info "Ardent connector probe passed"
    exit 0
  fi
  probe_require_mutation_allowed "creating an Ardent connector"
  case "${source_provider}" in
    postgresql|postgres|rds|supabase|self-hosted)
      source_url="${!connection_string_env:-}"
      [[ -n "${source_url}" ]] || probe_fail "${connection_string_env} must contain the source database URL"
      probe_info "creating Ardent Postgres connector without printing source URL"
      if ! create_output="$("${ardent}" "${create_args[@]}" "${source_url}" 2>&1)"; then
        printf '%s\n' "${create_output}" >&2
        probe_fail "Ardent connector creation failed"
      fi
      ;;
    neon)
      neon_api_key="${!neon_api_key_env:-}"
      [[ -n "${neon_api_key}" ]] || probe_fail "${neon_api_key_env} must contain a Neon API key"
      [[ -n "${neon_project_id}" ]] || probe_fail "THINWEDGE_NEON_PROJECT_ID or --neon-project-id must contain the Neon project id"
      neon_args=(connector create)
      [[ -n "${connector}" ]] && neon_args+=(--name "${connector}")
      neon_args+=(--byoc neon --api-key "${neon_api_key}" --project-id "${neon_project_id}" postgresql)
      probe_info "creating Ardent BYOC Neon connector without printing API key"
      if ! create_output="$("${ardent}" "${neon_args[@]}" 2>&1)"; then
        printf '%s\n' "${create_output}" >&2
        if grep -Fq "snapshot_max_connections" <<<"${create_output}"; then
          probe_fail "Ardent BYOC Neon setup hit the known server-side snapshot_max_connections failure; keep the connector deleted/clean and retry after Ardent fixes BYOC Neon"
        fi
        probe_fail "Ardent BYOC Neon connector creation failed"
      fi
      ;;
  esac
else
  probe_info "listing Ardent connectors"
  if ! listing="$(probe_maybe_dry_run "${ardent}" connector list 2>&1)"; then
    printf '%s\n' "${listing}" >&2
    if grep -Eiq 'not authenticated|ardent login|no organization found' <<<"${listing}"; then
      probe_fail "Ardent CLI is not authenticated; run ardent login"
    fi
    if grep -Eiq 'no current project set|project switch' <<<"${listing}"; then
      probe_fail "Ardent has no current project; run ardent project list and ardent project switch <name>"
    fi
    probe_fail "failed to list Ardent connectors"
  fi
  if grep -Eiq 'no connectors found' <<<"${listing}"; then
    printf '%s\n' "${listing}" >&2
    probe_fail "no Ardent connectors found; create one with check-ardent-connector.sh --create --allow-mutation after approval"
  fi
  if [[ -n "${connector}" ]]; then
    grep -F -- "${connector}" <<<"${listing}" >/dev/null || probe_fail "connector not found in Ardent connector list: ${connector}"
    probe_info "connector found: ${connector}"
  fi
fi
probe_info "Ardent connector probe passed"
