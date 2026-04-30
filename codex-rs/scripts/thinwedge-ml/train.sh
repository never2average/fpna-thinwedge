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
training_payload_json=$(thinwedge_validate_training_payload_json)
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

remote_entrypoint_path=$(
  THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" python3 - <<'PY'
import os
import posixpath

entrypoint = os.environ.get("THINWEDGE_MODEL_REPOSITORY_ENTRYPOINT") or "scripts/train.sh"
repo_root = os.environ["THINWEDGE_REMOTE_REPO_ROOT"]
print(posixpath.join(repo_root, entrypoint))
PY
)

remote_job_dir="${remote_workspace_path}/thinwedge/jobs/${THINWEDGE_JOB_ID}"
remote_payload_path="${remote_job_dir}/training-payload.json"
remote_training_manifest_path="${remote_job_dir}/training-manifest.json"
remote_eval_manifest_path="${remote_job_dir}/eval-manifest.json"
remote_codegen_manifest_path="${remote_job_dir}/generated-files.json"
remote_stdout_log_path="${remote_job_dir}/train.stdout.log"
remote_stderr_log_path="${remote_job_dir}/train.stderr.log"

payload_file=$(mktemp)
local_training_manifest=$(mktemp)
local_eval_manifest=$(mktemp)
local_codegen_manifest=$(mktemp)
exec_payload_file=$(mktemp)
trap 'rm -f "$payload_file" "$local_training_manifest" "$local_eval_manifest" "$local_codegen_manifest" "$exec_payload_file"' EXIT

printf '%s\n' "$training_payload_json" | thinwedge_write_json_file "$payload_file"

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

transport_mode=""
if [[ -n "$ssh_host" && -n "$ssh_port" ]]; then
  transport_mode="ssh"
elif [[ -n "$http_endpoint" && -n "$control_token" ]]; then
  transport_mode="http"
else
  printf 'ThinWedge Runpod session is missing both SSH attach details and HTTP control access\n' >&2
  exit 1
fi

codegen_requested=$(
  THINWEDGE_TRAINING_PAYLOAD_JSON="$training_payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_TRAINING_PAYLOAD_JSON"])
files = ((payload.get("codegen") or {}).get("files")) or []
print("true" if files else "false")
PY
)

remote_command=$(
  THINWEDGE_REMOTE_JOB_DIR="$remote_job_dir" \
  THINWEDGE_REMOTE_PAYLOAD_PATH="$remote_payload_path" \
  THINWEDGE_REMOTE_ENTRYPOINT_PATH="$remote_entrypoint_path" \
  THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH="$remote_training_manifest_path" \
  THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
  THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH="$remote_codegen_manifest_path" \
  THINWEDGE_REMOTE_STDOUT_LOG_PATH="$remote_stdout_log_path" \
  THINWEDGE_REMOTE_STDERR_LOG_PATH="$remote_stderr_log_path" \
  THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" \
  THINWEDGE_CODEGEN_REQUESTED="$codegen_requested" \
  python3 - <<'PY'
import os
import shlex

quoted = {
    key: shlex.quote(os.environ[key])
    for key in [
        "THINWEDGE_REMOTE_JOB_DIR",
        "THINWEDGE_REMOTE_PAYLOAD_PATH",
        "THINWEDGE_REMOTE_ENTRYPOINT_PATH",
        "THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH",
        "THINWEDGE_REMOTE_EVAL_MANIFEST_PATH",
        "THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH",
        "THINWEDGE_REMOTE_STDOUT_LOG_PATH",
        "THINWEDGE_REMOTE_STDERR_LOG_PATH",
        "THINWEDGE_REMOTE_REPO_ROOT",
    ]
}
command = (
    f"mkdir -p {quoted['THINWEDGE_REMOTE_JOB_DIR']} && "
    f"cd {quoted['THINWEDGE_REMOTE_REPO_ROOT']} && "
    f"THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH={quoted['THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH']} "
    f"THINWEDGE_REMOTE_EVAL_MANIFEST_PATH={quoted['THINWEDGE_REMOTE_EVAL_MANIFEST_PATH']} "
    f"THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH={quoted['THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH']} "
    f"THINWEDGE_JOB_ID={shlex.quote(os.environ['THINWEDGE_JOB_ID'])} "
    f"THINWEDGE_MODEL_ID={shlex.quote(os.environ['THINWEDGE_MODEL_ID'])} "
    f"THINWEDGE_ENVIRONMENT_ID={shlex.quote(os.environ['THINWEDGE_ENVIRONMENT_ID'])} "
    f"bash {quoted['THINWEDGE_REMOTE_ENTRYPOINT_PATH']} "
    f"--thinwedge-payload {quoted['THINWEDGE_REMOTE_PAYLOAD_PATH']} "
    f"--thinwedge-output-dir {quoted['THINWEDGE_REMOTE_JOB_DIR']} "
    f"> {quoted['THINWEDGE_REMOTE_STDOUT_LOG_PATH']} "
    f"2> {quoted['THINWEDGE_REMOTE_STDERR_LOG_PATH']}"
)
print(command)
PY
)

if [[ "$transport_mode" == "http" ]]; then
  thinwedge_runpod_wait_for_control_server "$http_endpoint" "$control_token" 120
  thinwedge_runpod_upload_repository_http "$THINWEDGE_MODEL_REPOSITORY_ROOT" "$http_endpoint" "$control_token" "$remote_repo_root"
  thinwedge_runpod_upload_file_http "$payload_file" "$http_endpoint" "$control_token" "$remote_payload_path"
else
  thinwedge_runpod_upload_repository_ssh "$THINWEDGE_MODEL_REPOSITORY_ROOT" "$ssh_host" "$ssh_port" "$remote_repo_root" "$ssh_user" "$ssh_key_path"
  thinwedge_runpod_upload_payload "$payload_file" "$ssh_host" "$ssh_port" "$remote_payload_path" "$ssh_user" "$ssh_key_path"
fi

if thinwedge_runpod_mock_enabled; then
  THINWEDGE_TRAINING_PAYLOAD_JSON="$training_payload_json" \
  THINWEDGE_REPOSITORY_JSON="$repository_json" \
  THINWEDGE_REMOTE_REPO_ROOT="$remote_repo_root" \
  THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH="$remote_training_manifest_path" \
  THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
  THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH="$remote_codegen_manifest_path" \
  THINWEDGE_REMOTE_STDOUT_LOG_PATH="$remote_stdout_log_path" \
  THINWEDGE_REMOTE_STDERR_LOG_PATH="$remote_stderr_log_path" \
  THINWEDGE_REMOTE_COMMAND="$remote_command" \
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" \
  python3 - <<'PY'
import json
import os
import pathlib
import time

payload = json.loads(os.environ["THINWEDGE_TRAINING_PAYLOAD_JSON"])
repository = json.loads(os.environ["THINWEDGE_REPOSITORY_JSON"])
session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
mock_root = pathlib.Path(os.environ["THINWEDGE_RUNPOD_MOCK_DIR"]) / "uploaded"
remote_repo_root = pathlib.Path(os.environ["THINWEDGE_REMOTE_REPO_ROOT"].lstrip("/"))
training_manifest_path = mock_root / os.environ["THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH"].lstrip("/")
eval_manifest_path = mock_root / os.environ["THINWEDGE_REMOTE_EVAL_MANIFEST_PATH"].lstrip("/")
codegen_manifest_path = mock_root / os.environ["THINWEDGE_REMOTE_CODEGEN_MANIFEST_PATH"].lstrip("/")
stdout_log_path = mock_root / os.environ["THINWEDGE_REMOTE_STDOUT_LOG_PATH"].lstrip("/")
stderr_log_path = mock_root / os.environ["THINWEDGE_REMOTE_STDERR_LOG_PATH"].lstrip("/")
for path in [training_manifest_path, eval_manifest_path, codegen_manifest_path, stdout_log_path, stderr_log_path]:
    path.parent.mkdir(parents=True, exist_ok=True)

generated_files = []
for entry in ((payload.get("codegen") or {}).get("files")) or []:
    target_path = mock_root / remote_repo_root / entry["path"]
    target_path.parent.mkdir(parents=True, exist_ok=True)
    target_path.write_text(
        "# mock generated statistical model file\n"
        f"# model_id: {os.environ['THINWEDGE_MODEL_ID']}\n"
        f"# instruction: {entry.get('instruction') or entry.get('prompt') or ''}\n",
        encoding="utf-8",
    )
    generated_files.append({
        "path": entry["path"],
        "remotePath": f"{os.environ['THINWEDGE_REMOTE_REPO_ROOT']}/{entry['path']}",
        "mode": "mock",
    })

training_manifest = {
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": session.get("podId"),
    "workspacePath": session.get("workspacePath"),
    "remoteCommand": os.environ.get("THINWEDGE_REMOTE_COMMAND"),
    "repository": repository,
    "payload": payload,
    "generatedFiles": generated_files,
    "status": "completed",
    "generatedAt": int(time.time()),
}
eval_manifest = {
    "id": f"eval-{os.environ['THINWEDGE_JOB_ID']}",
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "status": "completed",
    "summary": f"Training completed for {os.environ['THINWEDGE_MODEL_ID']}",
    "metrics": {
        "epochs": payload.get("epochs"),
        "learningRate": payload.get("learningRate"),
        "dataset": payload.get("dataset"),
        "generatedFileCount": len(generated_files),
    },
    "artifactPaths": [os.environ["THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH"]],
    "createdAt": int(time.time()),
}
training_manifest_path.write_text(json.dumps(training_manifest, indent=2) + "\n", encoding="utf-8")
eval_manifest_path.write_text(json.dumps(eval_manifest, indent=2) + "\n", encoding="utf-8")
codegen_manifest_path.write_text(json.dumps({"generatedFiles": generated_files}, indent=2) + "\n", encoding="utf-8")
stdout_log_path.write_text("mock training completed\n", encoding="utf-8")
stderr_log_path.write_text("", encoding="utf-8")
PY
fi

if [[ "$transport_mode" == "http" ]]; then
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
  thinwedge_runpod_http_download_file "$http_endpoint" "$control_token" "$remote_training_manifest_path" "$local_training_manifest"
  thinwedge_runpod_http_download_file "$http_endpoint" "$control_token" "$remote_eval_manifest_path" "$local_eval_manifest"
  thinwedge_runpod_http_download_file "$http_endpoint" "$control_token" "$remote_codegen_manifest_path" "$local_codegen_manifest"
else
  thinwedge_runpod_remote_exec_ssh "$ssh_host" "$ssh_port" "$remote_command" "$ssh_user" "$ssh_key_path" >/dev/null
  thinwedge_runpod_download_manifest "$ssh_host" "$ssh_port" "$remote_training_manifest_path" "$local_training_manifest" "$ssh_user" "$ssh_key_path"
  thinwedge_runpod_download_manifest "$ssh_host" "$ssh_port" "$remote_eval_manifest_path" "$local_eval_manifest" "$ssh_user" "$ssh_key_path"
  thinwedge_runpod_download_manifest "$ssh_host" "$ssh_port" "$remote_codegen_manifest_path" "$local_codegen_manifest" "$ssh_user" "$ssh_key_path"
fi

artifact_path="$(thinwedge_artifacts_dir)/training/${THINWEDGE_JOB_ID}.json"
THINWEDGE_LOCAL_TRAINING_MANIFEST="$local_training_manifest" python3 - <<'PY' | thinwedge_write_json_file "$artifact_path"
import json
import os

with open(os.environ["THINWEDGE_LOCAL_TRAINING_MANIFEST"], encoding="utf-8") as handle:
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
print(payload.get("summary") or f"Training completed for {os.environ['THINWEDGE_MODEL_ID']}")
PY
)

thinwedge_write_eval_json \
  "$eval_id" \
  "$eval_summary" \
  "completed" \
  "$metrics_json" \
  "$remote_training_manifest_path"

generated_files_json=$(
  THINWEDGE_LOCAL_CODEGEN_MANIFEST="$local_codegen_manifest" python3 - <<'PY'
import json
import os

with open(os.environ["THINWEDGE_LOCAL_CODEGEN_MANIFEST"], encoding="utf-8") as handle:
    payload = json.load(handle)
print(json.dumps(payload.get("generatedFiles") or []))
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

THINWEDGE_GENERATED_FILES_JSON="$generated_files_json" \
THINWEDGE_REMOTE_COMMAND="$remote_command" \
THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH="$remote_training_manifest_path" \
THINWEDGE_REMOTE_EVAL_MANIFEST_PATH="$remote_eval_manifest_path" \
THINWEDGE_POD_ID="$pod_id" \
THINWEDGE_WORKSPACE_PATH="$remote_workspace_path" \
python3 - <<'PY'
import json
import os

payload = {
    "tool": "statisticalmodels.submitJob",
    "jobId": os.environ["THINWEDGE_JOB_ID"],
    "jobType": "training",
    "modelId": os.environ["THINWEDGE_MODEL_ID"],
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": os.environ["THINWEDGE_POD_ID"],
    "workspacePath": os.environ["THINWEDGE_WORKSPACE_PATH"],
    "remoteCommand": os.environ["THINWEDGE_REMOTE_COMMAND"],
    "artifactManifestPath": os.environ["THINWEDGE_REMOTE_TRAINING_MANIFEST_PATH"],
    "evalManifestPath": os.environ["THINWEDGE_REMOTE_EVAL_MANIFEST_PATH"],
    "generatedFiles": json.loads(os.environ["THINWEDGE_GENERATED_FILES_JSON"]),
    "status": "completed",
}
print(json.dumps(payload))
PY
