#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/runpod.sh"

thinwedge_require_command python3
thinwedge_require_env THINWEDGE_ENVIRONMENT_ID
thinwedge_require_env THINWEDGE_ACTION
thinwedge_require_env THINWEDGE_AGENT_ROLE

environment_json=$(thinwedge_runpod_environment_record_json)
runpod_config_json=$(thinwedge_validate_runpod_environment_config_json "$(thinwedge_runpod_environment_config_json)")
session_json=$(thinwedge_runpod_read_session)
session_path=$(thinwedge_runpod_session_path)

cached_pod_id=$(
  THINWEDGE_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_SESSION_JSON"])
print(session.get("podId") or "")
PY
)

launch_disposition="created"
pod_json=""

if [[ -n "$cached_pod_id" ]]; then
  if pod_json=$(thinwedge_runpod_get_pod "$cached_pod_id" 2>/dev/null); then
    current_status=$(thinwedge_runpod_pod_status "$pod_json")
    runtime_ready=$(thinwedge_runpod_runtime_ready "$pod_json")
    if [[ "$current_status" == "RUNNING" && "$runtime_ready" == "true" ]]; then
      launch_disposition="reused"
    elif [[ ",STOPPED,EXITED,CREATED," == *",$current_status,"* ]]; then
      thinwedge_runpod_start_pod "$cached_pod_id" >/dev/null
      pod_json=$(thinwedge_runpod_wait_for_runtime_ready "$cached_pod_id" "$(THINWEDGE_RUNPOD_STARTUP_TIMEOUT_SEC=${THINWEDGE_RUNPOD_STARTUP_TIMEOUT_SEC:-}; printf '%s' "${THINWEDGE_RUNPOD_STARTUP_TIMEOUT_SEC:-$(
        THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config.get("startupTimeoutSec", 900))
PY
      )}")")
      launch_disposition="started"
    else
      pod_json=""
    fi
  fi
fi

if [[ -z "$pod_json" ]]; then
  create_payload_json=$(thinwedge_runpod_prepare_create_payload_json "$runpod_config_json")
  pod_json=$(thinwedge_runpod_create_pod "$create_payload_json")
  pod_id=$(
    THINWEDGE_RUNPOD_POD_JSON="$pod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
print(pod["id"])
PY
  )
  pod_json=$(thinwedge_runpod_wait_for_runtime_ready "$pod_id" "$(
    THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config.get("startupTimeoutSec", 900))
PY
  )")
fi

session_status=$(thinwedge_runpod_session_status "$pod_json")
attach_json=$(thinwedge_runpod_attach_snippets_json "$pod_json" "$runpod_config_json")
control_token=$(thinwedge_runpod_control_token_from_config "$runpod_config_json")
control_server_available=$(
  THINWEDGE_CONTROL_TOKEN="$control_token" \
  THINWEDGE_ATTACH_JSON="$attach_json" python3 - <<'PY'
import json
import os

attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
print("true" if os.environ["THINWEDGE_CONTROL_TOKEN"] and attach.get("httpEndpoint") else "false")
PY
)
if [[ "$control_server_available" == "true" ]]; then
  control_startup_timeout=$(
    THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config.get("startupTimeoutSec", 900))
PY
  )
  control_endpoint=$(
    THINWEDGE_ATTACH_JSON="$attach_json" python3 - <<'PY'
import json
import os

attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
print(attach.get("httpEndpoint") or "")
PY
  )
  thinwedge_runpod_wait_for_control_server "$control_endpoint" "$control_token" "$control_startup_timeout"
fi

wait_for_public_ssh=$(
  THINWEDGE_CONTROL_SERVER_AVAILABLE="$control_server_available" \
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(
    "true"
    if os.environ["THINWEDGE_CONTROL_SERVER_AVAILABLE"] != "true"
    and config.get("supportsSsh")
    and config.get("supportPublicIp", True)
    else "false"
)
PY
)
if [[ "$wait_for_public_ssh" == "true" ]]; then
  network_timeout_sec=$(
    THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
print(config.get("startupTimeoutSec", 900))
PY
  )
  network_deadline=$(( $(thinwedge_timestamp) + network_timeout_sec ))
  pod_id=$(
    THINWEDGE_RUNPOD_POD_JSON="$pod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
print(pod["id"])
PY
  )
  while true; do
    public_ssh_ready=$(
      THINWEDGE_ATTACH_JSON="$attach_json" python3 - <<'PY'
import json
import os

attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
print("true" if attach.get("sshHost") and attach.get("sshPort") else "false")
PY
    )
    if [[ "$public_ssh_ready" == "true" ]]; then
      break
    fi
    if (( $(thinwedge_timestamp) >= network_deadline )); then
      printf 'Timed out waiting for Runpod Pod %s to publish public SSH connection details\n' "$pod_id" >&2
      exit 1
    fi
    sleep 5
    pod_json=$(thinwedge_runpod_get_pod "$pod_id")
    attach_json=$(thinwedge_runpod_attach_snippets_json "$pod_json" "$runpod_config_json")
  done
fi

THINWEDGE_RUNPOD_POD_JSON="$pod_json" \
THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" \
THINWEDGE_SESSION_JSON="$session_json" \
THINWEDGE_ENVIRONMENT_JSON="$environment_json" \
THINWEDGE_ATTACH_JSON="$attach_json" \
THINWEDGE_LAUNCH_DISPOSITION="$launch_disposition" \
THINWEDGE_SESSION_STATUS="$session_status" \
THINWEDGE_SESSION_PATH="$session_path" \
python3 - <<'PY' | thinwedge_runpod_write_session
import json
import os
import time

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
previous = json.loads(os.environ["THINWEDGE_SESSION_JSON"])
environment = json.loads(os.environ["THINWEDGE_ENVIRONMENT_JSON"])
attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
session = {
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": pod.get("id"),
    "templateId": pod.get("templateId") or config.get("templateId"),
    "podName": pod.get("name") or config.get("name"),
    "status": os.environ["THINWEDGE_SESSION_STATUS"],
    "desiredStatus": (pod.get("desiredStatus") or "RUNNING").lower(),
    "workspacePath": config.get("workspacePath"),
    "volumeMountPath": pod.get("volumeMountPath") or config.get("volumeMountPath"),
    "publicIp": pod.get("publicIp"),
    "httpEndpoint": attach.get("httpEndpoint"),
    "portMappings": pod.get("portMappings") or {},
    "supportsSsh": config.get("supportsSsh"),
    "sshPrivateKeyPath": config.get("sshPrivateKeyPath"),
    "attach": attach,
    "lastRemoteSyncAt": int(time.time()),
    "launchDisposition": os.environ["THINWEDGE_LAUNCH_DISPOSITION"],
    "stopMode": config.get("stopMode", "stop"),
    "contextPath": os.environ.get("THINWEDGE_CONTEXT_JSON"),
    "rawPod": pod,
    "environment": environment,
}
if previous.get("createdAt"):
    session["createdAt"] = previous["createdAt"]
else:
    session["createdAt"] = int(time.time())
print(json.dumps(session))
PY

THINWEDGE_RUNPOD_POD_JSON="$pod_json" \
THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" \
THINWEDGE_ATTACH_JSON="$attach_json" \
THINWEDGE_LAUNCH_DISPOSITION="$launch_disposition" \
THINWEDGE_SESSION_STATUS="$session_status" \
THINWEDGE_SESSION_PATH="$session_path" \
python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
payload = {
    "tool": "trainingenvironments.launch",
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": pod.get("id"),
    "status": os.environ["THINWEDGE_SESSION_STATUS"],
    "httpEndpoint": attach.get("httpEndpoint"),
    "attach": attach,
    "workspacePath": config.get("workspacePath"),
    "sessionPath": os.environ["THINWEDGE_SESSION_PATH"],
    "launchDisposition": os.environ["THINWEDGE_LAUNCH_DISPOSITION"],
}
print(json.dumps(payload))
PY
