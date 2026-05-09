#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-ardent-connector.sh [--ardent PATH] [--connector NAME] [--create --connection-string-env VAR] [--dry-run] [--allow-mutation]

Checks Ardent connector readiness without exposing the source DB URL. By default
it lists connectors and optionally verifies that NAME appears in the listing.
Connector creation is mutation-gated and reads the source URL only from the named
environment variable.
EOF
}

ardent="${THINWEDGE_ARDENT_CLI:-ardent}"
connector="${THINWEDGE_ARDENT_CONNECTOR:-}"
create=0
connection_string_env="THINWEDGE_ARDENT_SOURCE_DATABASE_URL"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ardent) ardent="${2:?missing ardent path}"; shift 2 ;;
    --connector) connector="${2:?missing connector}"; shift 2 ;;
    --create) create=1; shift ;;
    --connection-string-env) connection_string_env="${2:?missing env var}"; shift 2 ;;
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
  if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
    probe_info "dry-run: ${ardent} connector create postgresql <redacted-source-url>"
    probe_info "Ardent connector probe passed"
    exit 0
  fi
  probe_require_mutation_allowed "creating an Ardent connector"
  source_url="${!connection_string_env:-}"
  [[ -n "${source_url}" ]] || probe_fail "${connection_string_env} must contain the source database URL"
  probe_info "creating Ardent Postgres connector without printing source URL"
  "${ardent}" connector create postgresql "${source_url}" >/dev/null
else
  probe_info "listing Ardent connectors"
  listing="$(probe_maybe_dry_run "${ardent}" connector list)"
  if [[ -n "${connector}" ]]; then
    grep -F -- "${connector}" <<<"${listing}" >/dev/null || probe_fail "connector not found in Ardent connector list: ${connector}"
    probe_info "connector found: ${connector}"
  fi
fi
probe_info "Ardent connector probe passed"
