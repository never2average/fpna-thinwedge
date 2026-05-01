#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/runpod.sh"

thinwedge_require_command python3
thinwedge_require_env THINWEDGE_MODEL_ID
thinwedge_require_env THINWEDGE_ACTION
thinwedge_require_env THINWEDGE_AGENT_ROLE
thinwedge_require_env THINWEDGE_JOB_ID
thinwedge_require_env THINWEDGE_ENVIRONMENT_ID

repository_json=$(thinwedge_repository_summary_json required)
batch_payload_json=$(thinwedge_validate_batch_inference_payload_json)
session_json=$(thinwedge_runpod_running_session_json)
runpod_config_json=$(thinwedge_validate_runpod_environment_config_json "$(thinwedge_runpod_environment_config_json)")

remote_repo_root=$(thinwedge_runpod_remote_repository_root "$runpod_config_json")
if [[ -z "$remote_repo_root" ]]; then
  printf 'ThinWedge Runpod config is missing remote `THINWEDGE_MODEL_REPOSITORY_ROOT`\n' >&2
  exit 1
fi

remote_workspace_path=$(
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config["workspacePath"])
PY
)

local_batch_entrypoint_path=$(
  THINWEDGE_LOCAL_REPOSITORY_ROOT="${THINWEDGE_MODEL_REPOSITORY_ROOT:-}" \
  THINWEDGE_BATCH_ENTRYPOINT="${THINWEDGE_MODEL_REPOSITORY_BATCH_ENTRYPOINT:-scripts/run_batch_inference.py}" \
  python3 - <<'PY'
import os
import pathlib

root = os.environ.get("THINWEDGE_LOCAL_REPOSITORY_ROOT") or ""
entrypoint = os.environ["THINWEDGE_BATCH_ENTRYPOINT"]
print(pathlib.Path(root, entrypoint) if root else "")
PY
)

if [[ -n "$local_batch_entrypoint_path" ]] && [[ ! -f "$local_batch_entrypoint_path" ]]; then
  printf 'ThinWedge local batch entrypoint does not exist: %s\n' "$local_batch_entrypoint_path" >&2
  exit 1
fi

remote_batch_entrypoint_path=$(
  THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" \
  THINWEDGE_BATCH_ENTRYPOINT="${THINWEDGE_MODEL_REPOSITORY_BATCH_ENTRYPOINT:-scripts/run_batch_inference.py}" \
  python3 - <<'PY'
import os
import posixpath

print(posixpath.join(os.environ["THINWEDGE_REMOTE_REPO_ROOT"], os.environ["THINWEDGE_BATCH_ENTRYPOINT"]))
PY
)

remote_job_dir="${remote_workspace_path}/thinwedge/jobs/${THINWEDGE_JOB_ID}"
remote_payload_path="${remote_job_dir}/batch-payload.json"
remote_artifact_manifest_path="${remote_job_dir}/batch-artifact-manifest.json"
remote_eval_manifest_path="${remote_job_dir}/batch-eval-manifest.json"
remote_stdout_log_path="${remote_job_dir}/batch.stdout.log"
remote_stderr_log_path="${remote_job_dir}/batch.stderr.log"

payload_file=$(mktemp)
local_artifact_manifest=$(mktemp)
local_eval_manifest=$(mktemp)
http_response_file=$(mktemp)
exec_payload_file=$(mktemp)
trap 'rm -f "$payload_file" "$local_artifact_manifest" "$local_eval_manifest" "$http_response_file" "$exec_payload_file"' EXIT

printf '%s\n' "$batch_payload_json" | thinwedge_write_json_file "$payload_file"

ssh_host=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
attach = session.get("attach") or {}
print(attach.get("sshHost") or "")
PY
)
ssh_port=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
attach = session.get("attach") or {}
print(attach.get("sshPort") or "")
PY
)
ssh_user=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
attach = session.get("attach") or {}
print(attach.get("sshUser") or "root")
PY
)
ssh_key_path=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
attach = session.get("attach") or {}
print(attach.get("sshKeyPath") or session.get("sshPrivateKeyPath") or "")
PY
)
http_endpoint=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
print(session.get("httpEndpoint") or "")
PY
)
control_token=$(thinwedge_runpod_control_token_from_config "$runpod_config_json")

requested_mode=$(
  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(payload.get("mode") or "auto")
PY
)

artifact_output_path="$(thinwedge_artifacts_dir)/batch-inference/${THINWEDGE_JOB_ID}.json"
execution_mode=""
remote_command=""

ssh_available="false"
if [[ -n "$ssh_host" && -n "$ssh_port" ]]; then
  ssh_available="true"
fi
http_control_available="false"
if [[ -n "$http_endpoint" && -n "$control_token" ]]; then
  http_control_available="true"
fi

if [[ "$ssh_available" == "false" && "$http_control_available" == "false" ]]; then
  printf 'ThinWedge Runpod session is missing both SSH attach details and HTTP control access\n' >&2
  exit 1
fi

maybe_mock_remote_batch_result() {
  local mode=$1
  local remote_output_path
  remote_output_path=$(
    THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(payload["outputPath"])
PY
  )

  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" \
  THINWEDGE_REPOSITORY_JSON="$repository_json" \
  THINWEDGE_REMOTE_OUTPUT_PATH="$remote_output_path" \
  THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH="$remote_artifact_manifest_path" \
  THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
  THINWEDGE_REMOTE_STDOUT_LOG_PATH="$remote_stdout_log_path" \
  THINWEDGE_REMOTE_STDERR_LOG_PATH="$remote_stderr_log_path" \
  THINWEDGE_REMOTE_COMMAND="$remote_command" \
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" \
  THINWEDGE_EXECUTION_MODE="$mode" \
  python3 - <<'PY'
import json
import os
import pathlib
import time

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
repository = json.loads(os.environ["THINWEDGE_REPOSITORY_JSON"])
session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
mock_root = pathlib.Path(os.environ["THINWEDGE_RUNPOD_MOCK_DIR"]) / "uploaded"
output_path = mock_root / os.environ["THINWEDGE_REMOTE_OUTPUT_PATH"].lstrip("/")
artifact_manifest_path = mock_root / os.environ["THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH"].lstrip("/")
eval_manifest_path = mock_root / os.environ["THINWEDGE_REMOTE_EVAL_MANIFEST_PATH"].lstrip("/")
stdout_log_path = mock_root / os.environ["THINWEDGE_REMOTE_STDOUT_LOG_PATH"].lstrip("/")
stderr_log_path = mock_root / os.environ["THINWEDGE_REMOTE_STDERR_LOG_PATH"].lstrip("/")
for path in [output_path, artifact_manifest_path, eval_manifest_path, stdout_log_path, stderr_log_path]:
    path.parent.mkdir(parents=True, exist_ok=True)

prediction_payload = {
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "prediction": "mocked",
    "executionMode": os.environ["THINWEDGE_EXECUTION_MODE"],
}
output_path.write_text(json.dumps([prediction_payload], indent=2) + "\n", encoding="utf-8")

artifact_manifest = {
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": session.get("podId"),
    "inputPath": payload.get("inputPath"),
    "inputUri": payload.get("inputUri"),
    "outputPath": payload.get("outputPath"),
    "rowCount": 1,
    "executionMode": os.environ["THINWEDGE_EXECUTION_MODE"],
    "repository": repository,
    "generatedAt": int(time.time()),
}
eval_manifest = {
    "id": f"eval-{os.environ['THINWEDGE_JOB_ID']}",
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "status": "completed",
    "summary": f"Batch inference completed for {os.environ['THINWEDGE_MODEL_ID']}",
    "metrics": {
        "batchSize": payload.get("batchSize"),
        "shardIndex": payload.get("shardIndex"),
        "shardCount": payload.get("shardCount"),
        "rowCount": 1,
        "executionMode": os.environ["THINWEDGE_EXECUTION_MODE"],
    },
    "artifactPaths": [os.environ["THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH"], payload.get("outputPath")],
    "createdAt": int(time.time()),
}
artifact_manifest_path.write_text(json.dumps(artifact_manifest, indent=2) + "\n", encoding="utf-8")
eval_manifest_path.write_text(json.dumps(eval_manifest, indent=2) + "\n", encoding="utf-8")
stdout_log_path.write_text("mock batch inference completed\n", encoding="utf-8")
stderr_log_path.write_text("", encoding="utf-8")
PY
}

if [[ "$requested_mode" != "ssh" && "$http_control_available" == "true" ]]; then
  execution_mode="http"
fi

if [[ -z "$execution_mode" ]]; then
  execution_mode="ssh"
fi

if [[ "$execution_mode" == "ssh" && "$ssh_available" != "true" ]]; then
  printf 'ThinWedge batch inference requested SSH execution but the Runpod session has no SSH attach details\n' >&2
  exit 1
fi

runtime_family=$(
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
env = config.get("env") or {}
print((env.get("THINWEDGE_RUNTIME_FAMILY") or "rapids").strip().lower())
PY
)

remote_command=$(
  THINWEDGE_REMOTE_JOB_DIR="$remote_job_dir" \
  THINWEDGE_REMOTE_PAYLOAD_PATH="$remote_payload_path" \
  THINWEDGE_REMOTE_BATCH_ENTRYPOINT_PATH="$remote_batch_entrypoint_path" \
  THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH="$remote_artifact_manifest_path" \
  THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
  THINWEDGE_REMOTE_STDOUT_LOG_PATH="$remote_stdout_log_path" \
  THINWEDGE_REMOTE_STDERR_LOG_PATH="$remote_stderr_log_path" \
  THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" \
  THINWEDGE_RUNTIME_FAMILY="$runtime_family" \
  python3 - <<'PY'
import os
import shlex

quoted = {
    key: shlex.quote(os.environ[key])
    for key in [
        "THINWEDGE_REMOTE_JOB_DIR",
        "THINWEDGE_REMOTE_PAYLOAD_PATH",
        "THINWEDGE_REMOTE_BATCH_ENTRYPOINT_PATH",
        "THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH",
        "THINWEDGE_REMOTE_EVAL_MANIFEST_PATH",
        "THINWEDGE_REMOTE_STDOUT_LOG_PATH",
        "THINWEDGE_REMOTE_STDERR_LOG_PATH",
        "THINWEDGE_REMOTE_REPO_ROOT",
    ]
}
command = (
    f"mkdir -p {quoted['THINWEDGE_REMOTE_JOB_DIR']} && "
    f"cd {quoted['THINWEDGE_REMOTE_REPO_ROOT']} && "
    f"THINWEDGE_REMOTE_BATCH_ARTIFACT_MANIFEST_PATH={quoted['THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH']} "
    f"THINWEDGE_REMOTE_BATCH_EVAL_MANIFEST_PATH={quoted['THINWEDGE_REMOTE_EVAL_MANIFEST_PATH']} "
    f"THINWEDGE_JOB_ID={shlex.quote(os.environ['THINWEDGE_JOB_ID'])} "
    f"THINWEDGE_MODEL_ID={shlex.quote(os.environ['THINWEDGE_MODEL_ID'])} "
    f"THINWEDGE_ENVIRONMENT_ID={shlex.quote(os.environ['THINWEDGE_ENVIRONMENT_ID'])} "
)
runtime_family = os.environ["THINWEDGE_RUNTIME_FAMILY"]
if runtime_family == "rapids":
    command += f"python -m cudf.pandas {quoted['THINWEDGE_REMOTE_BATCH_ENTRYPOINT_PATH']} "
else:
    command += f"python3 {quoted['THINWEDGE_REMOTE_BATCH_ENTRYPOINT_PATH']} "
command += (
    f"--thinwedge-payload {quoted['THINWEDGE_REMOTE_PAYLOAD_PATH']} "
    f"--thinwedge-output-dir {quoted['THINWEDGE_REMOTE_JOB_DIR']} "
    f"> {quoted['THINWEDGE_REMOTE_STDOUT_LOG_PATH']} "
    f"2> {quoted['THINWEDGE_REMOTE_STDERR_LOG_PATH']}"
)
print(command)
PY
)
remote_command=$(thinwedge_runpod_prepend_python_dependency_bootstrap "$runpod_config_json" "$remote_command")

if [[ "$execution_mode" == "http" ]]; then
  thinwedge_runpod_wait_for_control_server "$http_endpoint" "$control_token" 120
  thinwedge_runpod_upload_repository_http "$THINWEDGE_MODEL_REPOSITORY_ROOT" "$http_endpoint" "$control_token" "$remote_repo_root"
  thinwedge_runpod_upload_file_http "$payload_file" "$http_endpoint" "$control_token" "$remote_payload_path"
  if thinwedge_runpod_mock_enabled; then
    maybe_mock_remote_batch_result "$execution_mode"
  else
    THINWEDGE_REMOTE_COMMAND="$remote_command" \
    THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" \
    THINWEDGE_REMOTE_TIMEOUT_SEC="${THINWEDGE_RUNPOD_REMOTE_TIMEOUT_SEC:-900}" \
    python3 - <<'PY' | thinwedge_write_json_file "$exec_payload_file"
import json
import os

payload = {
    "command": os.environ["THINWEDGE_REMOTE_COMMAND"],
    "cwd": os.environ["THINWEDGE_REMOTE_REPO_ROOT"],
    "timeoutSec": int(os.environ["THINWEDGE_REMOTE_TIMEOUT_SEC"]),
}
print(json.dumps(payload))
PY
    thinwedge_runpod_exec_http "$http_endpoint" "$control_token" "$exec_payload_file" >/dev/null
  fi
  thinwedge_runpod_http_download_file "$http_endpoint" "$control_token" "$remote_artifact_manifest_path" "$local_artifact_manifest"
  thinwedge_runpod_http_download_file "$http_endpoint" "$control_token" "$remote_eval_manifest_path" "$local_eval_manifest"
else
  thinwedge_runpod_upload_repository_ssh "$THINWEDGE_MODEL_REPOSITORY_ROOT" "$ssh_host" "$ssh_port" "$remote_repo_root" "$ssh_user" "$ssh_key_path"
  thinwedge_runpod_upload_payload "$payload_file" "$ssh_host" "$ssh_port" "$remote_payload_path" "$ssh_user" "$ssh_key_path"
  if thinwedge_runpod_mock_enabled; then
    maybe_mock_remote_batch_result "$execution_mode"
  fi
  thinwedge_runpod_remote_exec_ssh "$ssh_host" "$ssh_port" "$remote_command" "$ssh_user" "$ssh_key_path" >/dev/null
  thinwedge_runpod_download_manifest "$ssh_host" "$ssh_port" "$remote_artifact_manifest_path" "$local_artifact_manifest" "$ssh_user" "$ssh_key_path"
  thinwedge_runpod_download_manifest "$ssh_host" "$ssh_port" "$remote_eval_manifest_path" "$local_eval_manifest" "$ssh_user" "$ssh_key_path"
fi

THINWEDGE_LOCAL_ARTIFACT_MANIFEST="$local_artifact_manifest" python3 - <<'PY' | thinwedge_write_json_file "$artifact_output_path"
import json
import os

with open(os.environ["THINWEDGE_LOCAL_ARTIFACT_MANIFEST"], encoding="utf-8") as handle:
    payload = json.load(handle)
print(json.dumps(payload))
PY

metrics_json=$(
  THINWEDGE_LOCAL_EVAL_MANIFEST="$local_eval_manifest" python3 - <<'PY'
import json
import os

with open(os.environ["THINWEDGE_LOCAL_EVAL_MANIFEST"], encoding="utf-8") as handle:
    payload = json.load(handle)
print(json.dumps(payload.get("metrics") or {}))
PY
)

eval_id=$(
  THINWEDGE_LOCAL_EVAL_MANIFEST="$local_eval_manifest" python3 - <<'PY'
import json
import os

with open(os.environ["THINWEDGE_LOCAL_EVAL_MANIFEST"], encoding="utf-8") as handle:
    payload = json.load(handle)
print(payload.get("id") or f"eval-{os.environ['THINWEDGE_JOB_ID']}")
PY
)
eval_summary=$(
  THINWEDGE_LOCAL_EVAL_MANIFEST="$local_eval_manifest" python3 - <<'PY'
import json
import os

with open(os.environ["THINWEDGE_LOCAL_EVAL_MANIFEST"], encoding="utf-8") as handle:
    payload = json.load(handle)
print(payload.get("summary") or f"Batch inference completed for {os.environ['THINWEDGE_MODEL_ID']}")
PY
)

thinwedge_write_eval_json \
  "$eval_id" \
  "$eval_summary" \
  "completed" \
  "$metrics_json" \
  "$remote_artifact_manifest_path"

shard_json=$(
  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(json.dumps({
    "index": payload.get("shardIndex"),
    "count": payload.get("shardCount"),
}))
PY
)
pod_id=$(
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
print(session.get("podId") or "")
PY
)
input_path=$(
  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(payload.get("inputPath") or "")
PY
)
input_uri=$(
  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(payload.get("inputUri") or "")
PY
)
output_path=$(
  THINWEDGE_BATCH_PAYLOAD_JSON="$batch_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
print(payload.get("outputPath") or "")
PY
)

THINWEDGE_SHARD_JSON="$shard_json" \
THINWEDGE_REMOTE_COMMAND="$remote_command" \
THINWEDGE_HTTP_ENDPOINT="$http_endpoint" \
THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH="$remote_artifact_manifest_path" \
THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
THINWEDGE_POD_ID="$pod_id" \
THINWEDGE_EXECUTION_MODE="$execution_mode" \
THINWEDGE_INPUT_PATH="$input_path" \
THINWEDGE_INPUT_URI="$input_uri" \
THINWEDGE_OUTPUT_PATH="$output_path" \
python3 - <<'PY'
import json
import os

payload = {
    "tool": "statisticalmodels.submitJob",
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "jobType": "batchInference",
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": os.environ["THINWEDGE_POD_ID"],
    "httpEndpoint": os.environ["THINWEDGE_HTTP_ENDPOINT"] or None,
    "executionMode": os.environ["THINWEDGE_EXECUTION_MODE"],
    "remoteCommand": os.environ["THINWEDGE_REMOTE_COMMAND"] or None,
    "inputPath": os.environ["THINWEDGE_INPUT_PATH"] or None,
    "inputUri": os.environ["THINWEDGE_INPUT_URI"] or None,
    "outputPath": os.environ["THINWEDGE_OUTPUT_PATH"],
    "artifactManifestPath": os.environ["THINWEDGE_REMOTE_ARTIFACT_MANIFEST_PATH"],
    "evalManifestPath": os.environ["THINWEDGE_REMOTE_EVAL_MANIFEST_PATH"],
    "shard": json.loads(os.environ["THINWEDGE_SHARD_JSON"]),
    "status": "completed",
}
print(json.dumps(payload))
PY
