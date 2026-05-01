#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

thinwedge_require_command python3

live_runpod=0
if [[ "${1:-}" == "--live-runpod" ]]; then
  live_runpod=1
fi

temp_root=$(mktemp -d)
trap 'rm -rf "$temp_root"' EXIT

export THINWEDGE_HOME="$temp_root/.thinwedge"
export THINWEDGE_THINWEDGE_HOME="$THINWEDGE_HOME"
workspace_root="$temp_root/workspace"
mkdir -p "$workspace_root"

if [[ "$live_runpod" -eq 0 ]]; then
  export THINWEDGE_RUNPOD_MOCK_DIR="$temp_root/runpod-mock"
  mkdir -p "$THINWEDGE_RUNPOD_MOCK_DIR"
fi

"$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/seed-fixtures.sh" "$THINWEDGE_HOME" "$workspace_root" >/dev/null

model_repo_root="$workspace_root/model-pricing-repo"
context_path="$THINWEDGE_HOME/thinwedge/ml/runtime/smoke-context.json"
mkdir -p "$(dirname "$context_path")"

write_mock_response() {
  local method=$1
  local path=$2
  local body_json=$3

  THINWEDGE_MOCK_METHOD="$method" \
  THINWEDGE_MOCK_PATH="$path" \
  THINWEDGE_MOCK_BODY_JSON="$body_json" \
  python3 - <<'PY'
import json
import os
import pathlib

safe = f"{os.environ['THINWEDGE_MOCK_METHOD']}_{os.environ['THINWEDGE_MOCK_PATH']}".replace("/", "__").replace("?", "_").replace("&", "_")
path = pathlib.Path(os.environ["THINWEDGE_RUNPOD_MOCK_DIR"]) / f"{safe}.json"
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(json.loads(os.environ["THINWEDGE_MOCK_BODY_JSON"]), indent=2) + "\n", encoding="utf-8")
PY
}

pod_json() {
  local status=$1
  local desired_status=${2:-$status}
  THINWEDGE_POD_STATUS="$status" THINWEDGE_POD_DESIRED_STATUS="$desired_status" python3 - <<'PY'
import json
import os

payload = {
    "id": "pod-thinwedge-smoke",
    "name": "thinwedge-smoke-pod",
    "templateId": "tmpl-thinwedge-pricing",
    "status": os.environ["THINWEDGE_POD_STATUS"],
    "desiredStatus": os.environ["THINWEDGE_POD_DESIRED_STATUS"],
    "publicIp": "203.0.113.10",
    "runtime": {"ports": [{"privatePort": 22, "publicPort": 2202, "ip": "203.0.113.10", "isIpPublic": True}]},
    "uptimeSeconds": 120,
    "volumeMountPath": "/workspace",
    "portMappings": {
        "22": 2202,
        "8000": 18000
    }
}
print(json.dumps(payload))
PY
}

if [[ "$live_runpod" -eq 0 ]]; then
  write_mock_response "POST" "/pods" "$(pod_json "RUNNING")"
  write_mock_response "GET" "/pods/pod-thinwedge-smoke" "$(pod_json "RUNNING")"
  write_mock_response "POST" "/pods/pod-thinwedge-smoke/start" "$(pod_json "RUNNING")"
  write_mock_response "POST" "/pods/pod-thinwedge-smoke/stop" "$(pod_json "STOPPED")"
else
  thinwedge_require_env RUNPOD_API_KEY
fi

python3 - "$THINWEDGE_HOME/thinwedge/ml/statisticalmodels.json" "$THINWEDGE_HOME/thinwedge/ml/trainingenvironments.json" "$context_path" <<'PY'
import json
import pathlib
import sys

models_path = pathlib.Path(sys.argv[1])
environments_path = pathlib.Path(sys.argv[2])
context_path = pathlib.Path(sys.argv[3])
models = json.loads(models_path.read_text(encoding="utf-8"))["models"]
environments = json.loads(environments_path.read_text(encoding="utf-8"))["environments"]
payload = {
    "action": "smoke",
    "agentRole": "pricing_researcher",
    "model": models[0],
    "environment": environments[0],
    "job": {
        "id": "job-smoke",
        "type": "training"
    },
    "payload": {
        "epochs": 3,
        "learningRate": 0.01,
        "dataset": "pricing-v1",
        "codegen": {
            "files": [
                {
                    "path": "generated/model.py",
                    "language": "python",
                    "instruction": "Create a pricing model scaffold that exposes build_model(config)."
                }
            ]
        }
    }
}
context_path.parent.mkdir(parents=True, exist_ok=True)
context_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

export THINWEDGE_CONTEXT_JSON="$context_path"
export THINWEDGE_AGENT_ROLE="pricing_researcher"
export THINWEDGE_MODEL_ID="model-pricing"
export THINWEDGE_ENVIRONMENT_ID="env-pricing"
export THINWEDGE_MODEL_REPOSITORY_ROOT="$model_repo_root"
export THINWEDGE_MODEL_REPOSITORY_CONFIG="$model_repo_root/config.yaml"
export THINWEDGE_MODEL_REPOSITORY_REF="main"
export THINWEDGE_MODEL_REPOSITORY_ENTRYPOINT="scripts/train.sh"
export THINWEDGE_MODEL_REPOSITORY_BATCH_ENTRYPOINT="scripts/run_batch_inference.py"

launch_output=$(THINWEDGE_ACTION="launch" bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/launch.sh")
THINWEDGE_SMOKE_JSON="$launch_output" python3 -c '
import json
import os

payload = json.loads(os.environ["THINWEDGE_SMOKE_JSON"])
assert payload["tool"] == "trainingenvironments.launch"
assert payload["environmentId"] == "env-pricing"
assert payload["provider"] == "runpod"
assert payload["status"] == "running"
assert payload["launchDisposition"] == "created"
print("launch.sh OK")
'

if [[ "$live_runpod" -eq 0 ]]; then
  train_output=$(THINWEDGE_ACTION="submitTraining" THINWEDGE_JOB_ID="job-train-smoke" THINWEDGE_JOB_TYPE="training" THINWEDGE_PAYLOAD_JSON='{"epochs":3,"learningRate":0.01,"dataset":"pricing-v1","codegen":{"files":[{"path":"generated/model.py","language":"python","instruction":"Create a pricing model scaffold that exposes build_model(config)."}]}}' bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/train.sh")
  THINWEDGE_SMOKE_JSON="$train_output" python3 -c '
import json
import os

payload = json.loads(os.environ["THINWEDGE_SMOKE_JSON"])
assert payload["tool"] == "statisticalmodels.submitJob"
assert payload["jobType"] == "training"
assert payload["jobId"] == "job-train-smoke"
assert payload["modelId"] == "model-pricing"
assert payload["environmentId"] == "env-pricing"
assert payload["provider"] == "runpod"
assert payload["status"] == "completed"
assert payload["generatedFiles"][0]["path"] == "generated/model.py"
assert "pandas" in payload["remoteCommand"]
assert "numpy" in payload["remoteCommand"]
assert "matplotlib" in payload["remoteCommand"]
assert "wandb" in payload["remoteCommand"]
print("train.sh OK")
'

  train_failure_stderr=$(mktemp)
  if THINWEDGE_ACTION="submitTraining" THINWEDGE_JOB_ID="job-train-invalid" THINWEDGE_JOB_TYPE="training" THINWEDGE_PAYLOAD_JSON='{"codegen":{"files":[{"language":"python"}]}}' bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/train.sh" >/dev/null 2>"$train_failure_stderr"; then
    printf 'train.sh unexpectedly accepted an invalid payload\n' >&2
    rm -f "$train_failure_stderr"
    exit 1
  fi
  python3 - "$train_failure_stderr" <<'PY'
import pathlib
import sys

stderr_text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
assert "requires `path`" in stderr_text
print("train.sh validation failure OK")
PY
  rm -f "$train_failure_stderr"

  batch_output=$(THINWEDGE_ACTION="submitBatchInference" THINWEDGE_JOB_ID="job-batch-smoke" THINWEDGE_JOB_TYPE="batchInference" THINWEDGE_PAYLOAD_JSON='{"inputPath":"/workspace/thinwedge/datasets/pricing-input.json","outputPath":"/workspace/thinwedge/outputs/job-batch-smoke/predictions.json","batchSize":64}' bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/batch_infer.sh")
  THINWEDGE_SMOKE_JSON="$batch_output" python3 -c '
import json
import os

payload = json.loads(os.environ["THINWEDGE_SMOKE_JSON"])
assert payload["tool"] == "statisticalmodels.submitJob"
assert payload["jobType"] == "batchInference"
assert payload["jobId"] == "job-batch-smoke"
assert payload["modelId"] == "model-pricing"
assert payload["environmentId"] == "env-pricing"
assert payload["provider"] == "runpod"
assert payload["executionMode"] == "http"
assert payload["status"] == "completed"
assert payload["outputPath"] == "/workspace/thinwedge/outputs/job-batch-smoke/predictions.json"
assert "pandas" in payload["remoteCommand"]
assert "numpy" in payload["remoteCommand"]
assert "matplotlib" in payload["remoteCommand"]
assert "wandb" in payload["remoteCommand"]
print("batch_infer.sh OK")
'

  batch_failure_stderr=$(mktemp)
  if THINWEDGE_ACTION="submitBatchInference" THINWEDGE_JOB_ID="job-batch-invalid" THINWEDGE_JOB_TYPE="batchInference" THINWEDGE_PAYLOAD_JSON='{"inputPath":"/workspace/thinwedge/datasets/pricing-input.json","mode":"nope"}' bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/batch_infer.sh" >/dev/null 2>"$batch_failure_stderr"; then
    printf 'batch_infer.sh unexpectedly accepted an invalid payload\n' >&2
    rm -f "$batch_failure_stderr"
    exit 1
  fi
  python3 - "$batch_failure_stderr" <<'PY'
import pathlib
import sys

stderr_text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
assert "requires `outputPath`" in stderr_text or "must be one of `auto`, `http`, or `ssh`" in stderr_text
print("batch_infer.sh validation failure OK")
PY
  rm -f "$batch_failure_stderr"
else
  printf 'live Runpod mode: launch/attach/stop only; training and batch jobs are skipped unless the Pod image is prepared with the remote repositories and entrypoints ThinWedge expects\n'
fi

attach_output=$(THINWEDGE_ACTION="attach" bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/attach.sh")
THINWEDGE_SMOKE_JSON="$attach_output" python3 -c '
import json
import os

payload = json.loads(os.environ["THINWEDGE_SMOKE_JSON"])
assert payload["tool"] == "trainingenvironments.attach"
assert payload["status"] == "running"
print("attach.sh OK")
'

if [[ "$live_runpod" -eq 0 ]]; then
  write_mock_response "GET" "/pods/pod-thinwedge-smoke" "$(pod_json "STOPPED")"
fi
stop_output=$(THINWEDGE_ACTION="stop" bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/stop.sh")
THINWEDGE_SMOKE_JSON="$stop_output" python3 -c '
import json
import os

payload = json.loads(os.environ["THINWEDGE_SMOKE_JSON"])
assert payload["tool"] == "trainingenvironments.stop"
assert payload["environmentId"] == "env-pricing"
assert payload["status"] == "stopped"
print("stop.sh OK")
'

if [[ "$live_runpod" -eq 0 ]]; then
python3 - "$THINWEDGE_HOME" "$THINWEDGE_RUNPOD_MOCK_DIR" <<'PY'
import json
import pathlib
import sys

thinwedge_home = pathlib.Path(sys.argv[1])
mock_root = pathlib.Path(sys.argv[2])
env_path = thinwedge_home / "thinwedge" / "ml" / "environments" / "env-pricing.json"
evals_dir = thinwedge_home / "thinwedge" / "ml" / "evals"
generated_model_path = mock_root / "uploaded" / "workspace" / "thinwedge" / "model-pricing-repo" / "generated" / "model.py"
batch_output_path = mock_root / "uploaded" / "workspace" / "thinwedge" / "outputs" / "job-batch-smoke" / "predictions.json"
train_artifact_path = thinwedge_home / "thinwedge" / "ml" / "artifacts" / "training" / "job-train-smoke.json"
batch_artifact_path = thinwedge_home / "thinwedge" / "ml" / "artifacts" / "batch-inference" / "job-batch-smoke.json"

environment = json.loads(env_path.read_text(encoding="utf-8"))
assert environment["status"] == "stopped"
assert environment["provider"] == "runpod"
assert generated_model_path.exists()
assert batch_output_path.exists()

train_artifact = json.loads(train_artifact_path.read_text(encoding="utf-8"))
assert train_artifact["jobId"] == "job-train-smoke"
batch_artifact = json.loads(batch_artifact_path.read_text(encoding="utf-8"))
assert batch_artifact["jobId"] == "job-batch-smoke"
assert batch_artifact["executionMode"] == "http"

eval_files = sorted(evals_dir.glob("*.json"))
assert len(eval_files) == 2
for path in eval_files:
    payload = json.loads(path.read_text(encoding="utf-8"))
    assert payload["status"] == "completed"
print("artifact and eval checks OK")
PY
else
python3 - "$THINWEDGE_HOME" <<'PY'
import json
import pathlib
import sys

thinwedge_home = pathlib.Path(sys.argv[1])
env_path = thinwedge_home / "thinwedge" / "ml" / "environments" / "env-pricing.json"
environment = json.loads(env_path.read_text(encoding="utf-8"))
assert environment["provider"] == "runpod"
assert environment["status"] in {"stopped", "terminated"}
print("live lifecycle checks OK")
PY
fi

printf 'ThinWedge ML smoke test passed in %s\n' "$temp_root"
