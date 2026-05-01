#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/runpod.sh"

thinwedge_require_command python3
thinwedge_require_env THINWEDGE_ENVIRONMENT_ID
thinwedge_require_env THINWEDGE_ACTION
thinwedge_require_env THINWEDGE_AGENT_ROLE

runpod_config_json=$(thinwedge_validate_runpod_environment_config_json "$(thinwedge_runpod_environment_config_json)")
session_json=$(thinwedge_runpod_read_session)
session_path=$(thinwedge_runpod_session_path)

pod_id=$(
  THINWEDGE_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_SESSION_JSON"])
print(session.get("podId") or "")
PY
)

if [[ -z "$pod_id" ]]; then
  printf 'ThinWedge environment `%s` has no cached Runpod session\n' "$THINWEDGE_ENVIRONMENT_ID" >&2
  exit 1
fi

stop_mode="${THINWEDGE_RUNPOD_STOP_MODE:-$(
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config.get("stopMode", "stop"))
PY
)}"

if [[ "$stop_mode" == "terminate" ]]; then
  thinwedge_runpod_delete_pod "$pod_id" >/dev/null
  pod_json='{}'
else
  thinwedge_runpod_stop_pod "$pod_id" >/dev/null
  pod_json=$(thinwedge_runpod_poll_pod_status_set "$pod_id" "STOPPED,EXITED,TERMINATED")
fi

environment_json=$(thinwedge_runpod_environment_record_json)

THINWEDGE_RUNPOD_POD_JSON="$pod_json" \
THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" \
THINWEDGE_SESSION_JSON="$session_json" \
THINWEDGE_ENVIRONMENT_JSON="$environment_json" \
THINWEDGE_RUNPOD_STOP_MODE="$stop_mode" \
python3 - <<'PY' | thinwedge_runpod_write_session
import json
import os
import time

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
previous = json.loads(os.environ["THINWEDGE_SESSION_JSON"])
environment = json.loads(os.environ["THINWEDGE_ENVIRONMENT_JSON"])
payload = {
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": None if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else previous.get("podId"),
    "templateId": previous.get("templateId") or config.get("templateId"),
    "podName": previous.get("podName") or config.get("name"),
    "status": "terminated" if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else "stopped",
    "desiredStatus": "terminated" if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else "stopped",
    "workspacePath": previous.get("workspacePath") or config.get("workspacePath"),
    "volumeMountPath": previous.get("volumeMountPath") or config.get("volumeMountPath"),
    "publicIp": None if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else pod.get("publicIp"),
    "httpEndpoint": None if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else previous.get("httpEndpoint"),
    "portMappings": {} if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else (pod.get("portMappings") or {}),
    "supportsSsh": config.get("supportsSsh"),
    "attach": None if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else previous.get("attach"),
    "lastRemoteSyncAt": int(time.time()),
    "launchDisposition": previous.get("launchDisposition"),
    "stopMode": os.environ["THINWEDGE_RUNPOD_STOP_MODE"],
    "contextPath": os.environ.get("THINWEDGE_CONTEXT_JSON") or previous.get("contextPath"),
    "rawPod": pod,
    "environment": environment,
    "createdAt": previous.get("createdAt") or int(time.time()),
}
print(json.dumps(payload))
PY

THINWEDGE_RUNPOD_STOP_MODE="$stop_mode" \
THINWEDGE_RUNPOD_POD_ID="$pod_id" \
THINWEDGE_SESSION_PATH="$session_path" \
python3 - <<'PY'
import json
import os

payload = {
    "tool": "trainingenvironments.stop",
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": None if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else os.environ["THINWEDGE_RUNPOD_POD_ID"],
    "status": "terminated" if os.environ["THINWEDGE_RUNPOD_STOP_MODE"] == "terminate" else "stopped",
    "stopMode": os.environ["THINWEDGE_RUNPOD_STOP_MODE"],
    "sessionPath": os.environ["THINWEDGE_SESSION_PATH"],
}
print(json.dumps(payload))
PY
