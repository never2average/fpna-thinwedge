#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-ardent-auth.sh [--ardent PATH] [--dry-run] [--verbose]

Checks that the Ardent CLI is installed and authenticated. The default status
command is `ardent status`; override with THINWEDGE_ARDENT_STATUS_COMMAND when a
specific Ardent CLI version uses a different auth/status subcommand.
EOF
}

ardent="${THINWEDGE_ARDENT_CLI:-ardent}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ardent) ardent="${2:?missing ardent path}"; shift 2 ;;
    --dry-run) TW_PROBE_DRY_RUN=1; shift ;;
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

probe_info "checking Ardent CLI auth using ${ardent}"
probe_maybe_dry_run "${ardent}" --version >/dev/null || probe_warn "Ardent CLI did not support --version; continuing"
if [[ -n "${THINWEDGE_ARDENT_STATUS_COMMAND:-}" ]]; then
  status_output="$(probe_maybe_dry_run bash -lc "${THINWEDGE_ARDENT_STATUS_COMMAND}" 2>&1)"
else
  status_output="$(probe_maybe_dry_run "${ardent}" status 2>&1)"
fi
if [[ "${TW_PROBE_DRY_RUN}" != "1" ]]; then
  if grep -Eiq 'not authenticated|run:[[:space:]]*ardent login|no organization found' <<<"${status_output}"; then
    printf '%s\n' "${status_output}" >&2
    probe_fail "Ardent CLI is installed but not authenticated; run ardent login"
  fi
fi
probe_info "Ardent auth probe passed"
