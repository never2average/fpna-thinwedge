#!/usr/bin/env bash
# Shared helpers for ThinWedge external integration probes.

set -euo pipefail

TW_PROBE_DRY_RUN=${TW_PROBE_DRY_RUN:-0}
TW_PROBE_ALLOW_MUTATION=${TW_PROBE_ALLOW_MUTATION:-0}
TW_PROBE_VERBOSE=${TW_PROBE_VERBOSE:-0}

probe_info() {
  printf '[thinwedge-probe] %s\n' "$*"
}

probe_warn() {
  printf '[thinwedge-probe][warn] %s\n' "$*" >&2
}

probe_fail() {
  printf '[thinwedge-probe][fail] %s\n' "$*" >&2
  exit 1
}

probe_skip() {
  printf '[thinwedge-probe][skip] %s\n' "$*"
  exit 0
}

probe_require_cmd() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || probe_fail "missing required command: ${command_name}"
}

probe_maybe_dry_run() {
  if [[ "${TW_PROBE_DRY_RUN}" == "1" ]]; then
    probe_info "dry-run: $*"
    return 0
  fi
  if [[ "${TW_PROBE_VERBOSE}" == "1" ]]; then
    probe_info "+ $*"
  fi
  "$@"
}

probe_require_mutation_allowed() {
  local action="$1"
  if [[ "${TW_PROBE_ALLOW_MUTATION}" != "1" ]]; then
    probe_fail "${action} is mutation-gated; set TW_PROBE_ALLOW_MUTATION=1 after explicit approval"
  fi
}

probe_profile_args() {
  local profile="$1"
  if [[ -n "${profile}" ]]; then
    printf '%s\0%s\0' --profile "${profile}"
  fi
}

probe_region_args() {
  local region="$1"
  if [[ -n "${region}" ]]; then
    printf '%s\0%s\0' --region "${region}"
  fi
}

probe_parse_common_flags() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)
        TW_PROBE_DRY_RUN=1
        shift
        ;;
      --verbose)
        TW_PROBE_VERBOSE=1
        shift
        ;;
      --allow-mutation)
        TW_PROBE_ALLOW_MUTATION=1
        shift
        ;;
      *)
        printf '%s\n' "$@"
        return 0
        ;;
    esac
  done
}
