#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

usage() {
  cat <<'EOF'
Usage: check-ardent-branch-lifecycle.sh [--ardent PATH] [--connector NAME] [--branch NAME] [--dry-run] [--allow-mutation]

Creates, inspects, and deletes a temporary Ardent branch. This is mutation-gated
because it allocates branch resources, but it must never touch the source DB
directly. The script prints branch command output only; do not use it with source
DB credentials.
EOF
}

ardent="${THINWEDGE_ARDENT_CLI:-ardent}"
connector="${THINWEDGE_ARDENT_CONNECTOR:-}"
branch="${THINWEDGE_ARDENT_BRANCH:-thinwedge-probe-$(date -u +%Y%m%d%H%M%S)-$$}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ardent) ardent="${2:?missing ardent path}"; shift 2 ;;
    --connector) connector="${2:?missing connector}"; shift 2 ;;
    --branch) branch="${2:?missing branch}"; shift 2 ;;
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

[[ "${TW_PROBE_DRY_RUN}" == "1" ]] || probe_require_mutation_allowed "creating and deleting an Ardent branch"

if [[ -n "${connector}" ]]; then
  probe_info "selecting Ardent connector ${connector}"
  probe_maybe_dry_run "${ardent}" connector switch "${connector}"
fi

probe_info "creating Ardent branch ${branch}${connector:+ connector=${connector}}"
probe_maybe_dry_run "${ardent}" branch create "${branch}"
cleanup() {
  probe_info "deleting Ardent branch ${branch}"
  probe_maybe_dry_run "${ardent}" branch delete "${branch}" || true
}
trap cleanup EXIT
probe_maybe_dry_run "${ardent}" branch info "${branch}"
probe_info "Ardent branch lifecycle probe passed"
