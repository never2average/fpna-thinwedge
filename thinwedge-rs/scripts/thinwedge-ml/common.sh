#!/usr/bin/env bash
set -euo pipefail

thinwedge_script_dir() {
  cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

thinwedge_repo_root() {
  cd "$(thinwedge_script_dir)/../.." && pwd
}

thinwedge_thinwedge_home() {
  if [[ -n "${THINWEDGE_THINWEDGE_HOME:-}" ]]; then
    printf '%s\n' "$THINWEDGE_THINWEDGE_HOME"
  elif [[ -n "${THINWEDGE_HOME:-}" ]]; then
    printf '%s\n' "$THINWEDGE_HOME"
  else
    printf '%s/.thinwedge\n' "$HOME"
  fi
}

thinwedge_data_dir() {
  printf '%s/thinwedge/ml\n' "$(thinwedge_thinwedge_home)"
}

thinwedge_runtime_dir() {
  printf '%s/runtime\n' "$(thinwedge_data_dir)"
}

thinwedge_artifacts_dir() {
  printf '%s/artifacts\n' "$(thinwedge_data_dir)"
}

thinwedge_evals_dir() {
  printf '%s/evals\n' "$(thinwedge_data_dir)"
}

thinwedge_environments_dir() {
  printf '%s/environments\n' "$(thinwedge_data_dir)"
}

thinwedge_jobs_dir() {
  printf '%s/jobs\n' "$(thinwedge_data_dir)"
}

thinwedge_timestamp() {
  date +%s
}

thinwedge_uuid() {
  python3 - <<'PY'
import uuid
print(uuid.uuid4())
PY
}

thinwedge_require_command() {
  local command_name=$1
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'ThinWedge script requires `%s` in PATH\n' "$command_name" >&2
    exit 1
  fi
}

thinwedge_require_env() {
  local variable_name=$1
  if [[ -z "${!variable_name:-}" ]]; then
    printf 'ThinWedge script requires `%s`\n' "$variable_name" >&2
    exit 1
  fi
}

thinwedge_optional_env() {
  local variable_name=$1
  printf '%s' "${!variable_name:-}"
}

thinwedge_ensure_parent_dir() {
  local path=$1
  mkdir -p "$(dirname "$path")"
}

thinwedge_write_json_file() {
  local path=$1
  thinwedge_ensure_parent_dir "$path"
  python3 -c '
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = json.load(sys.stdin)
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
' "$path"
}

thinwedge_context_query() {
  local expression=$1
  if [[ -z "${THINWEDGE_CONTEXT_JSON:-}" ]] || [[ ! -f "$THINWEDGE_CONTEXT_JSON" ]]; then
    return 1
  fi

  python3 - "$THINWEDGE_CONTEXT_JSON" "$expression" <<'PY'
import json
import sys

context_path = sys.argv[1]
expression = sys.argv[2]
with open(context_path, encoding="utf-8") as handle:
    value = json.load(handle)

for part in expression.split("."):
    if not part:
        continue
    if not isinstance(value, dict):
        sys.exit(1)
    value = value.get(part)
    if value is None:
        sys.exit(1)

if isinstance(value, (dict, list)):
    print(json.dumps(value))
else:
    print(value)
PY
}

thinwedge_repository_summary_json() {
  local mode=${1:-optional}
  local root="${THINWEDGE_MODEL_REPOSITORY_ROOT:-}"
  local config_path="${THINWEDGE_MODEL_REPOSITORY_CONFIG:-}"
  local ref_name="${THINWEDGE_MODEL_REPOSITORY_REF:-}"
  local entrypoint="${THINWEDGE_MODEL_REPOSITORY_ENTRYPOINT:-}"

  if [[ -z "$root" ]]; then
    if [[ "$mode" == "required" ]]; then
      printf 'ThinWedge script requires `THINWEDGE_MODEL_REPOSITORY_ROOT`\n' >&2
      exit 1
    fi
    printf '%s\n' '{"status":"notConfigured"}'
    return 0
  fi

  if [[ ! -d "$root" ]]; then
    printf 'ThinWedge repository root does not exist: %s\n' "$root" >&2
    exit 1
  fi

  local git_repo=false
  local git_head=""
  local git_branch=""
  if git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git_repo=true
    git_head=$(git -C "$root" rev-parse HEAD 2>/dev/null || true)
    git_branch=$(git -C "$root" rev-parse --abbrev-ref HEAD 2>/dev/null || true)
  fi

  export THINWEDGE_REPOSITORY_ROOT="$root"
  export THINWEDGE_REPOSITORY_CONFIG_PATH="$config_path"
  export THINWEDGE_REPOSITORY_REF_NAME="$ref_name"
  export THINWEDGE_REPOSITORY_ENTRYPOINT_VALUE="$entrypoint"
  export THINWEDGE_REPOSITORY_GIT_REPO="$git_repo"
  export THINWEDGE_REPOSITORY_GIT_HEAD="$git_head"
  export THINWEDGE_REPOSITORY_GIT_BRANCH="$git_branch"

  python3 - <<'PY'
import json
import os
import pathlib

root = pathlib.Path(os.environ["THINWEDGE_REPOSITORY_ROOT"])
config_path = os.environ.get("THINWEDGE_REPOSITORY_CONFIG_PATH") or None
entrypoint = os.environ.get("THINWEDGE_REPOSITORY_ENTRYPOINT_VALUE") or None
payload = {
    "status": "verified",
    "root": str(root),
    "exists": root.exists(),
    "configPath": config_path,
    "configExists": pathlib.Path(config_path).exists() if config_path else False,
    "refName": os.environ.get("THINWEDGE_REPOSITORY_REF_NAME") or None,
    "entrypoint": entrypoint,
    "entrypointExists": pathlib.Path(root, entrypoint).exists() if entrypoint else False,
    "gitRepo": os.environ.get("THINWEDGE_REPOSITORY_GIT_REPO") == "true",
    "gitHead": os.environ.get("THINWEDGE_REPOSITORY_GIT_HEAD") or None,
    "gitBranch": os.environ.get("THINWEDGE_REPOSITORY_GIT_BRANCH") or None,
}
print(json.dumps(payload))
PY
}

thinwedge_write_eval_json() {
  local eval_id=$1
  local summary=$2
  local status=$3
  local metrics_json=$4
  local artifact_path=${5:-}
  local path
  path="$(thinwedge_evals_dir)/${eval_id}.json"

  THINWEDGE_EVAL_ID="$eval_id" \
  THINWEDGE_EVAL_SUMMARY="$summary" \
  THINWEDGE_EVAL_STATUS="$status" \
  THINWEDGE_EVAL_METRICS_JSON="$metrics_json" \
  THINWEDGE_EVAL_ARTIFACT_PATH="$artifact_path" \
  python3 - <<'PY' | thinwedge_write_json_file "$path"
import json
import os
import time

artifact_path = os.environ.get("THINWEDGE_EVAL_ARTIFACT_PATH") or None
payload = {
    "id": os.environ["THINWEDGE_EVAL_ID"],
    "modelId": os.environ.get("THINWEDGE_MODEL_ID") or None,
    "jobId": os.environ.get("THINWEDGE_JOB_ID") or None,
    "status": os.environ["THINWEDGE_EVAL_STATUS"],
    "createdAt": int(time.time()),
    "summary": os.environ["THINWEDGE_EVAL_SUMMARY"],
    "metrics": json.loads(os.environ["THINWEDGE_EVAL_METRICS_JSON"]),
    "artifactPaths": [artifact_path] if artifact_path else [],
}
print(json.dumps(payload))
PY
}

if [[ -z "${THINWEDGE_RUNPOD_HELPERS_LOADED:-}" ]]; then
  _thinwedge_common_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  _thinwedge_runpod_helpers_path="${_thinwedge_common_dir}/runpod.sh"
  if [[ -f "$_thinwedge_runpod_helpers_path" ]]; then
    THINWEDGE_RUNPOD_HELPERS_LOADED=1
    # shellcheck source=/dev/null
    source "$_thinwedge_runpod_helpers_path"
  fi
  unset _thinwedge_common_dir
  unset _thinwedge_runpod_helpers_path
fi
