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

pod_json=$(thinwedge_runpod_get_pod "$pod_id")
attach_json=$(thinwedge_runpod_attach_snippets_json "$pod_json" "$runpod_config_json")
environment_json=$(thinwedge_runpod_environment_record_json)
session_status=$(thinwedge_runpod_session_status "$pod_json")

THINWEDGE_RUNPOD_POD_JSON="$pod_json" \
THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_config_json" \
THINWEDGE_SESSION_JSON="$session_json" \
THINWEDGE_ENVIRONMENT_JSON="$environment_json" \
THINWEDGE_ATTACH_JSON="$attach_json" \
THINWEDGE_SESSION_STATUS="$session_status" \
python3 - <<'PY' | thinwedge_runpod_write_session
import json
import os
import time

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
previous = json.loads(os.environ["THINWEDGE_SESSION_JSON"])
environment = json.loads(os.environ["THINWEDGE_ENVIRONMENT_JSON"])
attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
payload = {
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": pod.get("id"),
    "templateId": pod.get("templateId") or previous.get("templateId") or config.get("templateId"),
    "podName": pod.get("name") or previous.get("podName") or config.get("name"),
    "status": os.environ["THINWEDGE_SESSION_STATUS"],
    "desiredStatus": (pod.get("desiredStatus") or pod.get("status") or "UNKNOWN").lower(),
    "workspacePath": previous.get("workspacePath") or config.get("workspacePath"),
    "volumeMountPath": pod.get("volumeMountPath") or previous.get("volumeMountPath") or config.get("volumeMountPath"),
    "publicIp": pod.get("publicIp"),
    "httpEndpoint": attach.get("httpEndpoint"),
    "portMappings": pod.get("portMappings") or {},
    "supportsSsh": config.get("supportsSsh"),
    "sshPrivateKeyPath": config.get("sshPrivateKeyPath") or previous.get("sshPrivateKeyPath"),
    "attach": attach,
    "lastRemoteSyncAt": int(time.time()),
    "launchDisposition": previous.get("launchDisposition"),
    "stopMode": previous.get("stopMode") or config.get("stopMode", "stop"),
    "contextPath": os.environ.get("THINWEDGE_CONTEXT_JSON") or previous.get("contextPath"),
    "rawPod": pod,
    "environment": environment,
    "createdAt": previous.get("createdAt") or int(time.time()),
}
print(json.dumps(payload))
PY

THINWEDGE_RUNPOD_POD_JSON="$pod_json" \
THINWEDGE_ATTACH_JSON="$attach_json" \
THINWEDGE_SESSION_STATUS="$session_status" \
THINWEDGE_SESSION_PATH="$session_path" \
python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
attach = json.loads(os.environ["THINWEDGE_ATTACH_JSON"])
payload = {
    "tool": "trainingenvironments.attach",
    "environmentId": os.environ["THINWEDGE_ENVIRONMENT_ID"],
    "provider": "runpod",
    "podId": pod.get("id"),
    "status": os.environ["THINWEDGE_SESSION_STATUS"],
    "httpEndpoint": attach.get("httpEndpoint"),
    "attach": attach,
    "sessionPath": os.environ["THINWEDGE_SESSION_PATH"],
}
print(json.dumps(payload))
PY
