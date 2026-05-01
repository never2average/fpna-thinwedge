# ThinWedge Runpod Template Spec

Last reviewed: April 30, 2026

This document defines the Runpod Pod template contract that ThinWedge uses for generated statistical-model code.

## Goals

- Keep Runpod Pods as the source of truth for training and stateful batch inference.
- Keep image/runtime selection explicit instead of letting generated code install arbitrary GPU stacks at runtime.
- Keep persistent runtime state under `/workspace`.

## Required Pod Capabilities

Every ThinWedge Runpod template must provide:

- one Linux `amd64` container image
- one exposed HTTP port for short statistical-model API calls
- SSH over exposed TCP for remote execution and large batch jobs
- persistent storage mounted at `/workspace`
- Python 3.11+ with the relevant ML stack preinstalled

ThinWedge assumes these ports and paths:

- HTTP inference port: `8000`
- SSH port: `22`
- persistent workspace root: `/workspace`

## Template Families

ThinWedge uses two template families.

### `thinwedge-rapids-cu12`

Use for:

- tabular statistical models
- pandas-heavy feature engineering
- `cudf.pandas`
- `cuml.accel`
- direct `cuML`

Recommended base image:

```dockerfile
FROM rapidsai/base:26.04-cuda12-py3.12
```

Rationale:

- RAPIDS packages stay version-aligned
- CUDA 12 is the safer baseline across Runpod GPU availability
- the image is better suited for generated tabular/dataframe-heavy code than a general PyTorch image

### `thinwedge-pytorch`

Use for:

- neural training
- tensor-heavy inference
- custom `torch.nn.Module` models
- FastAPI/Uvicorn statistical-model APIs backed by PyTorch

Recommended base image strategy:

- derive from a pinned `runpod/pytorch:<exact-tag>`
- use Ubuntu 24.04 variants
- do not use `:latest`

Known official example from Runpod docs:

```dockerfile
FROM runpod/pytorch:1.0.2-cu1281-torch280-ubuntu2404
```

ThinWedge policy:

- prefer a pinned tag checked into infra config
- update deliberately when CUDA/PyTorch compatibility is revalidated

## Required Filesystem Layout

Everything important must live under `/workspace`.

ThinWedge standard layout:

- `/workspace/thinwedge/model-repo`
- `/workspace/thinwedge/env-repo`
- `/workspace/thinwedge/jobs/<jobId>`
- `/workspace/thinwedge/checkpoints`
- `/workspace/thinwedge/outputs`
- `/workspace/thinwedge/evals`
- `/workspace/thinwedge/logs`
- `/workspace/thinwedge/models`
- `/workspace/thinwedge/datasets`

Use `/tmp` only for scratch files.

## Startup Modes

ThinWedge supports two startup patterns.

### Interactive / SSH-enabled mode

Use when:

- `train.sh` and `batch_infer.sh` need SSH execution
- developers need VS Code / Cursor / terminal access

The image should keep base services alive and make `sshd` available.

### Application-only mode

Use when:

- the Pod exists only to serve a short-lived HTTP statistical-model API
- no interactive access is needed

In this mode the image should use an explicit entrypoint/CMD and bind the API to `0.0.0.0:8000`.

## Runpod `metadata.runpod` Mapping

ThinWedge environment records should keep the Runpod knobs in `trainingenvironments.json -> environments[].metadata.runpod`.

Recommended v1 shape:

```json
{
  "templateId": "tmpl-thinwedge-rapids-cu12",
  "gpuCount": 1,
  "gpuTypeId": "NVIDIA A100 80GB PCIe",
  "volumeMountPath": "/workspace",
  "workspacePath": "/workspace/thinwedge/env-pricing",
  "exposedHttpPort": 8000,
  "supportsSsh": true,
  "name": "thinwedge-pricing-env",
  "cloudType": "SECURE",
  "containerDiskInGb": 50,
  "volumeInGb": 100,
  "supportPublicIp": true,
  "stopMode": "stop",
  "startupTimeoutSec": 900,
  "autoInstallPythonPackages": true,
  "pythonPackages": ["pandas", "numpy", "matplotlib", "wandb"],
  "env": {
    "THINWEDGE_MODEL_REPOSITORY_ROOT": "/workspace/thinwedge/model-repo",
    "THINWEDGE_ENV_REPOSITORY_ROOT": "/workspace/thinwedge/env-repo",
    "THINWEDGE_RUNTIME_FAMILY": "rapids"
  }
}
```

By default, ThinWedge prepends a Pod-side dependency bootstrap before statistical-model training and batch inference commands. The default package list is `pandas`, `numpy`, `matplotlib`, and `wandb`. Set `autoInstallPythonPackages` to `false` to disable this behavior, or override `pythonPackages` to pin a different package set.

Recommended environment variable conventions:

- `THINWEDGE_RUNTIME_FAMILY=rapids|pytorch`
- `THINWEDGE_MODEL_REPOSITORY_ROOT=/workspace/thinwedge/model-repo`
- `THINWEDGE_ENV_REPOSITORY_ROOT=/workspace/thinwedge/env-repo`
- `THINWEDGE_OUTPUT_ROOT=/workspace/thinwedge/outputs`
- `THINWEDGE_CHECKPOINT_ROOT=/workspace/thinwedge/checkpoints`

## Networking Contract

ThinWedge assumes:

- short API calls go through `https://<pod-id>-8000.proxy.runpod.net`
- long or large jobs do not rely on synchronous HTTP
- services bind to `0.0.0.0`
- authentication is enforced by the service, not assumed from the proxy

Because the Runpod proxy has a request timeout, ThinWedge treats HTTP as the preferred path only for short statistical-model API calls.

## Storage Rules

- Do not write important outputs outside `/workspace`.
- Treat container disk as ephemeral.
- If a network volume is used, treat it as the authoritative `/workspace`.
- Back up critical checkpoints and artifacts outside Runpod when they matter long-term.

## ThinWedge Policy

- `train.sh` always runs inside a Pod from one of these template families.
- `batch_infer.sh` prefers the Pod HTTP API, then falls back to SSH one-shot execution.
- Generated code must not install or replace major GPU frameworks at runtime.

## Sources

- Runpod Pods overview: https://docs.runpod.io/pods/overview
- Runpod custom Pod templates: https://docs.runpod.io/pods/templates/create-custom-template
- Runpod SSH: https://docs.runpod.io/pods/configuration/use-ssh
- Runpod VS Code / Cursor: https://docs.runpod.io/pods/configuration/connect-to-ide
- Runpod network volumes: https://docs.runpod.io/storage/network-volumes
- RAPIDS install guide: https://docs.rapids.ai/install/
