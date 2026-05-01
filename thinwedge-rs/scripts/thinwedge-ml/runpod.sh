#!/usr/bin/env bash
set -euo pipefail

thinwedge_runpod_api_base() {
  printf '%s\n' "${THINWEDGE_RUNPOD_API_BASE_URL:-https://rest.runpod.io/v1}"
}

thinwedge_runpod_session_path() {
  thinwedge_require_env THINWEDGE_ENVIRONMENT_ID
  printf '%s/%s.json\n' "$(thinwedge_environments_dir)" "$THINWEDGE_ENVIRONMENT_ID"
}

thinwedge_require_runpod_api_key() {
  thinwedge_require_command curl
  thinwedge_require_env RUNPOD_API_KEY
}

thinwedge_runpod_mock_dir() {
  printf '%s' "${THINWEDGE_RUNPOD_MOCK_DIR:-}"
}

thinwedge_runpod_mock_enabled() {
  [[ -n "$(thinwedge_runpod_mock_dir)" ]]
}

thinwedge_runpod_mock_response_path() {
  local method=$1
  local path=$2
  local mock_dir
  mock_dir=$(thinwedge_runpod_mock_dir)
  if [[ -z "$mock_dir" ]]; then
    printf 'ThinWedge Runpod mock mode is not enabled\n' >&2
    exit 1
  fi

  THINWEDGE_RUNPOD_METHOD="$method" THINWEDGE_RUNPOD_PATH="$path" python3 - <<'PY'
import os

method = os.environ["THINWEDGE_RUNPOD_METHOD"]
path = os.environ["THINWEDGE_RUNPOD_PATH"]
safe = f"{method}_{path}".replace("/", "__").replace("?", "_").replace("&", "_")
print(os.path.join(os.environ["THINWEDGE_RUNPOD_MOCK_DIR"], f"{safe}.json"))
PY
}

thinwedge_runpod_request() {
  local method=$1
  local path=$2
  local body_json=${3:-}

  if thinwedge_runpod_mock_enabled; then
    local mock_path
    mock_path=$(thinwedge_runpod_mock_response_path "$method" "$path")
    if [[ ! -f "$mock_path" ]]; then
      printf 'Missing Runpod mock response: %s\n' "$mock_path" >&2
      exit 1
    fi
    cat "$mock_path"
    return 0
  fi

  thinwedge_require_runpod_api_key

  local url
  url="$(thinwedge_runpod_api_base)$path"
  local response_file
  response_file=$(mktemp)
  local http_code

  if [[ -n "$body_json" ]]; then
    if ! http_code=$(
      curl \
        --silent \
        --show-error \
        --location \
        --output "$response_file" \
        --write-out '%{http_code}' \
        --request "$method" \
        --header "Authorization: Bearer $RUNPOD_API_KEY" \
        --header 'Content-Type: application/json' \
        --data "$body_json" \
        "$url"
    ); then
      local curl_exit=$?
      rm -f "$response_file"
      printf 'Runpod request failed: %s %s\n' "$method" "$url" >&2
      exit "$curl_exit"
    fi
  else
    if ! http_code=$(
      curl \
        --silent \
        --show-error \
        --location \
        --output "$response_file" \
        --write-out '%{http_code}' \
        --request "$method" \
        --header "Authorization: Bearer $RUNPOD_API_KEY" \
        "$url"
    ); then
      local curl_exit=$?
      rm -f "$response_file"
      printf 'Runpod request failed: %s %s\n' "$method" "$url" >&2
      exit "$curl_exit"
    fi
  fi

  if [[ "$http_code" != 2* ]]; then
    printf 'Runpod request returned HTTP %s for %s %s\n' "$http_code" "$method" "$url" >&2
    cat "$response_file" >&2
    rm -f "$response_file"
    exit 1
  fi

  cat "$response_file"
  rm -f "$response_file"
}

thinwedge_runpod_get_pod() {
  local pod_id=$1
  thinwedge_runpod_request GET "/pods/$pod_id"
}

thinwedge_runpod_create_pod() {
  local create_payload_json=$1
  thinwedge_runpod_request POST "/pods" "$create_payload_json"
}

thinwedge_runpod_start_pod() {
  local pod_id=$1
  thinwedge_runpod_request POST "/pods/$pod_id/start"
}

thinwedge_runpod_stop_pod() {
  local pod_id=$1
  thinwedge_runpod_request POST "/pods/$pod_id/stop"
}

thinwedge_runpod_delete_pod() {
  local pod_id=$1
  thinwedge_runpod_request DELETE "/pods/$pod_id"
}

thinwedge_runpod_extract_port_mapping() {
  local pod_json=$1
  local container_port=$2

  THINWEDGE_RUNPOD_POD_JSON="$pod_json" THINWEDGE_RUNPOD_CONTAINER_PORT="$container_port" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
port_mappings = pod.get("portMappings") or {}
value = port_mappings.get(os.environ["THINWEDGE_RUNPOD_CONTAINER_PORT"])
if value is None:
    value = port_mappings.get(str(os.environ["THINWEDGE_RUNPOD_CONTAINER_PORT"]))
if value is not None:
    print(value)
PY
}

thinwedge_runpod_build_http_endpoint() {
  local pod_json=$1
  local exposed_http_port=$2

  THINWEDGE_RUNPOD_POD_JSON="$pod_json" THINWEDGE_RUNPOD_EXPOSED_HTTP_PORT="$exposed_http_port" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
pod_id = pod.get("id")
exposed_port = os.environ["THINWEDGE_RUNPOD_EXPOSED_HTTP_PORT"]
if pod_id and exposed_port:
    print(f"https://{pod_id}-{exposed_port}.proxy.runpod.net")
PY
}

thinwedge_runpod_pod_status() {
  local pod_json=$1
  THINWEDGE_RUNPOD_POD_JSON="$pod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
print((pod.get("desiredStatus") or pod.get("status") or "").upper())
PY
}

thinwedge_runpod_runtime_ready() {
  local pod_json=$1
  THINWEDGE_RUNPOD_POD_JSON="$pod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
runtime = pod.get("runtime")
uptime_seconds = pod.get("uptimeSeconds") or 0
machine = pod.get("machine") or {}
port_mappings = pod.get("portMappings") or {}
public_ip = pod.get("publicIp") or ""
ready = bool(runtime) or uptime_seconds > 0 or bool(machine) or bool(port_mappings) or bool(public_ip)
print("true" if ready else "false")
PY
}

thinwedge_runpod_session_status() {
  local pod_json=$1
  THINWEDGE_RUNPOD_POD_JSON="$pod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
desired_status = (pod.get("desiredStatus") or pod.get("status") or "").upper()
runtime = pod.get("runtime")
uptime_seconds = pod.get("uptimeSeconds") or 0
machine = pod.get("machine") or {}
port_mappings = pod.get("portMappings") or {}
public_ip = pod.get("publicIp") or ""
runtime_ready = bool(runtime) or uptime_seconds > 0 or bool(machine) or bool(port_mappings) or bool(public_ip)

if desired_status == "RUNNING":
    print("running" if runtime_ready else "starting")
elif desired_status in {"STOPPED", "EXITED"}:
    print("stopped")
elif desired_status == "TERMINATED":
    print("terminated")
else:
    print((pod.get("status") or desired_status or "unknown").lower())
PY
}

thinwedge_runpod_wait_for_runtime_ready() {
  local pod_id=$1
  local timeout_sec=${2:-600}
  local poll_interval_sec=${3:-5}
  local deadline=$(( $(thinwedge_timestamp) + timeout_sec ))
  local pod_json
  local current_status
  local runtime_ready

  while true; do
    pod_json=$(thinwedge_runpod_get_pod "$pod_id")
    current_status=$(thinwedge_runpod_pod_status "$pod_json")
    runtime_ready=$(thinwedge_runpod_runtime_ready "$pod_json")
    if [[ "$current_status" == "RUNNING" && "$runtime_ready" == "true" ]]; then
      printf '%s\n' "$pod_json"
      return 0
    fi
    if (( $(thinwedge_timestamp) >= deadline )); then
      printf 'Timed out waiting for Runpod Pod %s runtime readiness; last status=%s runtimeReady=%s\n' "$pod_id" "$current_status" "$runtime_ready" >&2
      exit 1
    fi
    sleep "$poll_interval_sec"
  done
}

thinwedge_runpod_poll_pod_status() {
  local pod_id=$1
  local desired_status=$2
  local timeout_sec=${3:-600}
  local poll_interval_sec=${4:-5}
  local deadline=$(( $(thinwedge_timestamp) + timeout_sec ))
  local pod_json
  local current_status

  while true; do
    pod_json=$(thinwedge_runpod_get_pod "$pod_id")
    current_status=$(thinwedge_runpod_pod_status "$pod_json")
    if [[ "$current_status" == "$desired_status" ]]; then
      printf '%s\n' "$pod_json"
      return 0
    fi
    if (( $(thinwedge_timestamp) >= deadline )); then
      printf 'Timed out waiting for Runpod Pod %s to reach %s; last status=%s\n' "$pod_id" "$desired_status" "$current_status" >&2
      exit 1
    fi
    sleep "$poll_interval_sec"
  done
}

thinwedge_runpod_poll_pod_status_set() {
  local pod_id=$1
  local desired_status_csv=$2
  local timeout_sec=${3:-600}
  local poll_interval_sec=${4:-5}
  local deadline=$(( $(thinwedge_timestamp) + timeout_sec ))
  local pod_json
  local current_status

  while true; do
    if ! pod_json=$(thinwedge_runpod_get_pod "$pod_id" 2>/dev/null); then
      if [[ "$desired_status_csv" == *"DELETED"* ]]; then
        printf '%s\n' '{}'
        return 0
      fi
      printf 'Unable to fetch Runpod Pod %s while waiting for statuses %s\n' "$pod_id" "$desired_status_csv" >&2
      exit 1
    fi
    current_status=$(thinwedge_runpod_pod_status "$pod_json")
    if [[ ",$desired_status_csv," == *",$current_status,"* ]]; then
      printf '%s\n' "$pod_json"
      return 0
    fi
    if (( $(thinwedge_timestamp) >= deadline )); then
      printf 'Timed out waiting for Runpod Pod %s to reach one of [%s]; last status=%s\n' "$pod_id" "$desired_status_csv" "$current_status" >&2
      exit 1
    fi
    sleep "$poll_interval_sec"
  done
}

thinwedge_runpod_read_session() {
  local session_path
  session_path=$(thinwedge_runpod_session_path)
  if [[ -f "$session_path" ]]; then
    cat "$session_path"
  else
    printf '%s\n' '{}'
  fi
}

thinwedge_runpod_write_session() {
  local session_path
  session_path=$(thinwedge_runpod_session_path)
  thinwedge_write_json_file "$session_path"
}

thinwedge_runpod_environment_record_json() {
  if thinwedge_context_query "environment" >/dev/null 2>&1; then
    thinwedge_context_query "environment"
    return 0
  fi

  local session_json
  session_json=$(thinwedge_runpod_read_session)
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
environment = session.get("environment")
print(json.dumps(environment if isinstance(environment, dict) else {}))
PY
}

thinwedge_runpod_environment_config_json() {
  local environment_json
  environment_json=$(thinwedge_runpod_environment_record_json)
  THINWEDGE_RUNPOD_ENVIRONMENT_JSON="$environment_json" python3 - <<'PY'
import json
import os

environment = json.loads(os.environ["THINWEDGE_RUNPOD_ENVIRONMENT_JSON"])
metadata = environment.get("metadata") or {}
runpod = metadata.get("runpod") or {}
print(json.dumps(runpod))
PY
}

thinwedge_runpod_running_session_json() {
  local session_json
  session_json=$(thinwedge_runpod_read_session)
  THINWEDGE_RUNPOD_SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os

session = json.loads(os.environ["THINWEDGE_RUNPOD_SESSION_JSON"])
pod_id = session.get("podId")
status = (session.get("status") or "").lower()
if not pod_id:
    raise SystemExit("ThinWedge Runpod session is missing `podId`")
if status != "running":
    raise SystemExit(f"ThinWedge Runpod session is not running; status={status or 'unknown'}")
print(json.dumps(session))
PY
}

thinwedge_runpod_remote_repository_root() {
  local runpod_json=$1
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
env = config.get("env") or {}
print(env.get("THINWEDGE_MODEL_REPOSITORY_ROOT") or "")
PY
}

thinwedge_validate_runpod_environment_config_json() {
  local runpod_json=$1
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
required = [
    "gpuCount",
    "volumeMountPath",
    "workspacePath",
    "exposedHttpPort",
    "supportsSsh",
]
missing = [field for field in required if config.get(field) in (None, "")]
if missing:
    raise SystemExit(f"ThinWedge Runpod config is missing required fields: {', '.join(missing)}")
if not config.get("templateId") and not config.get("imageName"):
    raise SystemExit("ThinWedge Runpod config requires either `templateId` or `imageName`")

normalized = {
    "templateId": config.get("templateId"),
    "gpuCount": config["gpuCount"],
    "volumeMountPath": config["volumeMountPath"],
    "workspacePath": config["workspacePath"],
    "exposedHttpPort": config["exposedHttpPort"],
    "supportsSsh": bool(config["supportsSsh"]),
    "name": config.get("name"),
    "imageName": config.get("imageName"),
    "cloudType": config.get("cloudType", "SECURE"),
    "gpuTypeId": config.get("gpuTypeId"),
    "dockerEntrypoint": config.get("dockerEntrypoint") or [],
    "dockerStartCmd": config.get("dockerStartCmd") or [],
    "globalNetworking": bool(config.get("globalNetworking", False)),
    "containerDiskInGb": config.get("containerDiskInGb"),
    "volumeInGb": config.get("volumeInGb"),
    "networkVolumeId": config.get("networkVolumeId"),
    "dataCenterIds": config.get("dataCenterIds") or [],
    "supportPublicIp": bool(config.get("supportPublicIp", True)),
    "sshPrivateKeyPath": config.get("sshPrivateKeyPath"),
    "dockerArgs": config.get("dockerArgs") or [],
    "env": config.get("env") or {},
    "stopMode": config.get("stopMode", "stop"),
    "startupTimeoutSec": config.get("startupTimeoutSec", 900),
}
print(json.dumps(normalized))
PY
}

thinwedge_runpod_prepare_create_payload_json() {
  local runpod_json=$1
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
payload = {
    "name": config.get("name"),
    "templateId": config.get("templateId"),
    "gpuCount": config.get("gpuCount"),
    "cloudType": config.get("cloudType", "SECURE"),
    "volumeMountPath": config.get("volumeMountPath"),
    "supportPublicIp": config.get("supportPublicIp", True),
}

optional_direct_fields = {
    "imageName": "imageName",
    "dockerEntrypoint": "dockerEntrypoint",
    "dockerStartCmd": "dockerStartCmd",
    "globalNetworking": "globalNetworking",
    "containerDiskInGb": "containerDiskInGb",
    "volumeInGb": "volumeInGb",
    "networkVolumeId": "networkVolumeId",
    "dockerArgs": "dockerArgs",
    "env": "env",
}
for source, target in optional_direct_fields.items():
    value = config.get(source)
    if value not in (None, "", [], {}):
        payload[target] = value

gpu_type_id = config.get("gpuTypeId")
if gpu_type_id:
    payload["gpuTypeIds"] = [gpu_type_id]

data_center_ids = config.get("dataCenterIds") or []
if data_center_ids:
    payload["dataCenterIds"] = data_center_ids

image_name = config.get("imageName")
if image_name and not config.get("templateId"):
    payload["ports"] = [f"{config['exposedHttpPort']}/http"]
    if config.get("supportsSsh"):
        payload["ports"].append("22/tcp")

payload = {key: value for key, value in payload.items() if value not in (None, "")}
print(json.dumps(payload))
PY
}

thinwedge_runpod_attach_snippets_json() {
  local pod_json=$1
  local runpod_json=$2

  THINWEDGE_RUNPOD_POD_JSON="$pod_json" THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

pod = json.loads(os.environ["THINWEDGE_RUNPOD_POD_JSON"])
config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
pod_id = pod.get("id")
public_ip = pod.get("publicIp")
port_mappings = pod.get("portMappings") or {}
ssh_port = port_mappings.get("22") or port_mappings.get(22)
attach = {
    "httpEndpoint": f"https://{pod_id}-{config['exposedHttpPort']}.proxy.runpod.net" if pod_id else None,
    "sshCommand": None,
    "sshHost": None,
    "sshPort": None,
    "sshUser": None,
    "sshKeyPath": config.get("sshPrivateKeyPath"),
    "runpodctl": f"runpodctl ssh {pod_id}" if pod_id else None,
    "vscodeHint": None,
    "cursorHint": None,
}
if config.get("supportsSsh") and public_ip and ssh_port:
    attach["sshHost"] = public_ip
    attach["sshPort"] = ssh_port
    attach["sshUser"] = "root"
    if config.get("sshPrivateKeyPath"):
        attach["sshCommand"] = f"ssh -i {config['sshPrivateKeyPath']} -o IdentitiesOnly=yes root@{public_ip} -p {ssh_port}"
    else:
        attach["sshCommand"] = f"ssh root@{public_ip} -p {ssh_port}"
    attach["vscodeHint"] = f"Host {pod_id}\\n  HostName {public_ip}\\n  User root\\n  Port {ssh_port}"
    attach["cursorHint"] = f"ssh root@{public_ip} -p {ssh_port}"
print(json.dumps(attach))
PY
}

thinwedge_runpod_control_token_from_config() {
  local runpod_json=$1
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
env = config.get("env") or {}
print(env.get("THINWEDGE_CONTROL_TOKEN") or "")
PY
}

thinwedge_runpod_control_port_from_config() {
  local runpod_json=$1
  THINWEDGE_RUNPOD_CONFIG_JSON="$runpod_json" python3 - <<'PY'
import json
import os

config = json.loads(os.environ["THINWEDGE_RUNPOD_CONFIG_JSON"])
env = config.get("env") or {}
print(env.get("THINWEDGE_CONTROL_PORT") or config.get("exposedHttpPort") or 8000)
PY
}

thinwedge_runpod_wait_for_control_server() {
  local http_endpoint=$1
  local control_token=$2
  local timeout_sec=${3:-60}

  if thinwedge_runpod_mock_enabled; then
    return 0
  fi

  thinwedge_require_command curl
  local deadline=$(( $(thinwedge_timestamp) + timeout_sec ))
  local health_url="${http_endpoint%/}/health"

  while true; do
    http_code=$(
      curl \
        --silent \
        --show-error \
        --output /dev/null \
        --write-out '%{http_code}' \
        --header "X-ThinWedge-Token: $control_token" \
        "$health_url" || true
    )
    if [[ "$http_code" == 2* ]]; then
      return 0
    fi
    if (( $(thinwedge_timestamp) >= deadline )); then
      printf 'Timed out waiting for ThinWedge control server at %s\n' "$health_url" >&2
      exit 1
    fi
    sleep 2
  done
}

thinwedge_runpod_http_post_binary() {
  local http_endpoint=$1
  local control_token=$2
  local path=$3
  local source_path=$4
  shift 4

  thinwedge_require_command curl
  local url="${http_endpoint%/}${path}"
  local extra_headers=()
  while [[ $# -gt 0 ]]; do
    extra_headers+=(--header "$1")
    shift
  done

  local response_file
  response_file=$(mktemp)
  local http_code
  if ! http_code=$(
    curl \
      --silent \
      --show-error \
      --location \
      --output "$response_file" \
      --write-out '%{http_code}' \
      --request POST \
      --header "X-ThinWedge-Token: $control_token" \
      --data-binary "@$source_path" \
      "${extra_headers[@]}" \
      "$url"
  ); then
    local curl_exit=$?
    rm -f "$response_file"
    printf 'ThinWedge control request failed: POST %s\n' "$url" >&2
    exit "$curl_exit"
  fi
  if [[ "$http_code" != 2* ]]; then
    printf 'ThinWedge control request returned HTTP %s for POST %s\n' "$http_code" "$url" >&2
    cat "$response_file" >&2
    rm -f "$response_file"
    exit 1
  fi
  cat "$response_file"
  rm -f "$response_file"
}

thinwedge_runpod_http_post_json() {
  local http_endpoint=$1
  local control_token=$2
  local path=$3
  local payload_file=$4

  thinwedge_require_command curl
  local url="${http_endpoint%/}${path}"
  local response_file
  response_file=$(mktemp)
  local http_code
  if ! http_code=$(
    curl \
      --silent \
      --show-error \
      --location \
      --output "$response_file" \
      --write-out '%{http_code}' \
      --request POST \
      --header "X-ThinWedge-Token: $control_token" \
      --header 'Content-Type: application/json' \
      --data "@$payload_file" \
      "$url"
  ); then
    local curl_exit=$?
    rm -f "$response_file"
    printf 'ThinWedge control request failed: POST %s\n' "$url" >&2
    exit "$curl_exit"
  fi
  if [[ "$http_code" != 2* ]]; then
    printf 'ThinWedge control request returned HTTP %s for POST %s\n' "$http_code" "$url" >&2
    cat "$response_file" >&2
    rm -f "$response_file"
    exit 1
  fi
  cat "$response_file"
  rm -f "$response_file"
}

thinwedge_runpod_mock_uploaded_dir() {
  printf '%s/uploaded\n' "$(thinwedge_runpod_mock_dir)"
}

thinwedge_runpod_http_download_file() {
  local http_endpoint=$1
  local control_token=$2
  local remote_path=$3
  local local_path=$4

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_uploaded_dir)"
    cp "$mock_upload_dir/$remote_path" "$local_path"
    return 0
  fi

  thinwedge_require_command curl
  local url="${http_endpoint%/}/thinwedge/file"
  local http_code
  if ! http_code=$(
    curl \
      --silent \
      --show-error \
      --location \
      --output "$local_path" \
      --write-out '%{http_code}' \
      --get \
      --data-urlencode "path=$remote_path" \
      --header "X-ThinWedge-Token: $control_token" \
      "$url"
  ); then
    local curl_exit=$?
    rm -f "$local_path"
    printf 'ThinWedge control request failed: GET %s\n' "$url" >&2
    exit "$curl_exit"
  fi
  if [[ "$http_code" != 2* ]]; then
    printf 'ThinWedge control request returned HTTP %s for GET %s\n' "$http_code" "$url" >&2
    cat "$local_path" >&2 || true
    rm -f "$local_path"
    exit 1
  fi
}

thinwedge_runpod_upload_repository_http() {
  local repository_root=$1
  local http_endpoint=$2
  local control_token=$3
  local destination=$4

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_uploaded_dir)"
    THINWEDGE_MOCK_SOURCE_ROOT="$repository_root" THINWEDGE_MOCK_DESTINATION="$mock_upload_dir/$destination" python3 - <<'PY'
import os
import pathlib
import shutil

source = pathlib.Path(os.environ["THINWEDGE_MOCK_SOURCE_ROOT"])
destination = pathlib.Path(os.environ["THINWEDGE_MOCK_DESTINATION"])
destination.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(source, destination, dirs_exist_ok=True)
PY
    return 0
  fi

  thinwedge_require_command tar
  local archive_path
  archive_path=$(mktemp)
  tar -C "$repository_root" -czf "$archive_path" .
  thinwedge_runpod_http_post_binary \
    "$http_endpoint" \
    "$control_token" \
    "/thinwedge/repository" \
    "$archive_path" \
    "X-ThinWedge-Destination: $destination" >/dev/null
  rm -f "$archive_path"
}

thinwedge_runpod_upload_repository_ssh() {
  local repository_root=$1
  local ssh_host=$2
  local ssh_port=$3
  local remote_path=$4
  local ssh_user=${5:-root}
  local ssh_key_path=${6:-}

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_uploaded_dir)"
    THINWEDGE_MOCK_SOURCE_ROOT="$repository_root" THINWEDGE_MOCK_DESTINATION="$mock_upload_dir/$remote_path" python3 - <<'PY'
import os
import pathlib
import shutil

source = pathlib.Path(os.environ["THINWEDGE_MOCK_SOURCE_ROOT"])
destination = pathlib.Path(os.environ["THINWEDGE_MOCK_DESTINATION"])
destination.parent.mkdir(parents=True, exist_ok=True)
shutil.copytree(source, destination, dirs_exist_ok=True)
PY
    return 0
  fi

  thinwedge_require_command tar
  thinwedge_require_command ssh
  local ssh_args=()
  if [[ -n "$ssh_key_path" ]]; then
    ssh_args=(-i "$ssh_key_path" -o IdentitiesOnly=yes)
  fi
  ssh_args+=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
  tar -C "$repository_root" -cf - . | ssh "${ssh_args[@]}" -p "$ssh_port" "${ssh_user}@${ssh_host}" "mkdir -p '$remote_path' && tar -C '$remote_path' -xf -"
}

thinwedge_runpod_upload_file_http() {
  local source_path=$1
  local http_endpoint=$2
  local control_token=$3
  local destination=$4

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_uploaded_dir)"
    mkdir -p "$(dirname "$mock_upload_dir/$destination")"
    cp "$source_path" "$mock_upload_dir/$destination"
    return 0
  fi

  thinwedge_runpod_http_post_binary \
    "$http_endpoint" \
    "$control_token" \
    "/thinwedge/file" \
    "$source_path" \
    "X-ThinWedge-Path: $destination" >/dev/null
}

thinwedge_runpod_exec_http() {
  local http_endpoint=$1
  local control_token=$2
  local payload_file=$3

  if thinwedge_runpod_mock_enabled; then
    THINWEDGE_EXEC_PAYLOAD_FILE="$payload_file" python3 - <<'PY'
import json
import os
import pathlib

payload = json.loads(pathlib.Path(os.environ["THINWEDGE_EXEC_PAYLOAD_FILE"]).read_text(encoding="utf-8"))
print(json.dumps({"status": "mocked", "command": payload.get("command"), "exitCode": 0}))
PY
    return 0
  fi

  thinwedge_runpod_http_post_json "$http_endpoint" "$control_token" "/thinwedge/exec" "$payload_file"
}

thinwedge_runpod_remote_exec_ssh() {
  local ssh_host=$1
  local ssh_port=$2
  local remote_command=$3
  local ssh_user=${4:-root}
  local ssh_key_path=${5:-}

  if thinwedge_runpod_mock_enabled; then
    THINWEDGE_RUNPOD_REMOTE_COMMAND="$remote_command" THINWEDGE_RUNPOD_SSH_HOST="$ssh_host" THINWEDGE_RUNPOD_SSH_PORT="$ssh_port" THINWEDGE_RUNPOD_SSH_USER="$ssh_user" python3 - <<'PY'
import json
import os

print(json.dumps({
    "status": "mocked",
    "sshHost": os.environ["THINWEDGE_RUNPOD_SSH_HOST"],
    "sshPort": os.environ["THINWEDGE_RUNPOD_SSH_PORT"],
    "sshUser": os.environ["THINWEDGE_RUNPOD_SSH_USER"],
    "remoteCommand": os.environ["THINWEDGE_RUNPOD_REMOTE_COMMAND"],
}))
PY
    return 0
  fi

  thinwedge_require_command ssh
  local ssh_args=()
  if [[ -n "$ssh_key_path" ]]; then
    ssh_args=(-i "$ssh_key_path" -o IdentitiesOnly=yes)
  fi
  ssh_args+=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
  ssh "${ssh_args[@]}" -p "$ssh_port" "${ssh_user}@${ssh_host}" "$remote_command"
}

thinwedge_runpod_upload_payload() {
  local source_path=$1
  local ssh_host=$2
  local ssh_port=$3
  local remote_path=$4
  local ssh_user=${5:-root}
  local ssh_key_path=${6:-}

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_dir)/uploaded"
    mkdir -p "$(dirname "$mock_upload_dir/$remote_path")"
    cp "$source_path" "$mock_upload_dir/$remote_path"
    return 0
  fi

  thinwedge_require_command scp
  thinwedge_require_command ssh
  local scp_args=()
  local ssh_args=()
  if [[ -n "$ssh_key_path" ]]; then
    scp_args=(-i "$ssh_key_path" -o IdentitiesOnly=yes)
    ssh_args=(-i "$ssh_key_path" -o IdentitiesOnly=yes)
  fi
  scp_args+=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
  ssh_args+=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
  local remote_dir=${remote_path%/*}
  if [[ "$remote_dir" == "$remote_path" ]]; then
    remote_dir='.'
  fi
  ssh "${ssh_args[@]}" -p "$ssh_port" "${ssh_user}@${ssh_host}" "mkdir -p '$remote_dir'"
  scp "${scp_args[@]}" -P "$ssh_port" "$source_path" "${ssh_user}@${ssh_host}:$remote_path"
}

thinwedge_runpod_download_manifest() {
  local ssh_host=$1
  local ssh_port=$2
  local remote_path=$3
  local local_path=$4
  local ssh_user=${5:-root}
  local ssh_key_path=${6:-}

  if thinwedge_runpod_mock_enabled; then
    local mock_upload_dir
    mock_upload_dir="$(thinwedge_runpod_mock_dir)/uploaded"
    cp "$mock_upload_dir/$remote_path" "$local_path"
    return 0
  fi

  thinwedge_require_command scp
  local scp_args=()
  if [[ -n "$ssh_key_path" ]]; then
    scp_args=(-i "$ssh_key_path" -o IdentitiesOnly=yes)
  fi
  scp_args+=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
  scp "${scp_args[@]}" -P "$ssh_port" "${ssh_user}@${ssh_host}:$remote_path" "$local_path"
}

thinwedge_validate_training_payload_json() {
  local payload_json="${1:-${THINWEDGE_PAYLOAD_JSON:-"{}"}}"
  THINWEDGE_TRAINING_PAYLOAD_JSON="$payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_TRAINING_PAYLOAD_JSON"])
codegen = payload.get("codegen") or {}
files = codegen.get("files") or []
for entry in files:
    if not entry.get("path"):
        raise SystemExit("Every training codegen.files entry requires `path`")
normalized = {
    "epochs": payload.get("epochs"),
    "learningRate": payload.get("learningRate"),
    "dataset": payload.get("dataset"),
    "codegen": {"files": files},
    "raw": payload,
}
print(json.dumps(normalized))
PY
}

thinwedge_validate_batch_inference_payload_json() {
  local payload_json="${1:-${THINWEDGE_PAYLOAD_JSON:-"{}"}}"
  THINWEDGE_BATCH_PAYLOAD_JSON="$payload_json" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["THINWEDGE_BATCH_PAYLOAD_JSON"])
if not payload.get("inputPath") and not payload.get("inputUri"):
    raise SystemExit("Batch inference payload requires `inputPath` or `inputUri`")
if not payload.get("outputPath"):
    raise SystemExit("Batch inference payload requires `outputPath`")
mode = payload.get("mode") or "auto"
if mode not in {"auto", "http", "ssh"}:
    raise SystemExit("Batch inference payload `mode` must be one of `auto`, `http`, or `ssh`")
normalized = {
    "inputPath": payload.get("inputPath"),
    "inputUri": payload.get("inputUri"),
    "outputPath": payload.get("outputPath"),
    "shardIndex": payload.get("shardIndex"),
    "shardCount": payload.get("shardCount"),
    "batchSize": payload.get("batchSize"),
    "timeoutSec": payload.get("timeoutSec"),
    "mode": mode,
    "raw": payload,
}
print(json.dumps(normalized))
PY
}
