# ThinWedge GPU PyTorch Codegen Spec

Last reviewed: April 30, 2026

This document defines how ThinWedge should generate GPU PyTorch code for Runpod Pods.

## Scope

Use this spec when the target model is:

- tensor-heavy
- neural
- sequence-oriented
- custom `torch.nn.Module` based
- not naturally modeled as a sklearn/cuML workflow

Default runtime family:

- `thinwedge-pytorch`

## Required Runtime Assumptions

Generated code must assume:

- it runs inside a pinned Runpod PyTorch image
- CUDA is already installed and validated by the image
- model data, checkpoints, outputs, and logs belong under `/workspace`

Generated code must not:

- install or replace PyTorch at runtime
- install random CUDA toolkits
- assume CPU fallback is acceptable unless ThinWedge explicitly allows it

## Required Bootstrap Pattern

Generated training code must fail fast if no GPU is available.

Required startup check:

```python
import torch

if not torch.cuda.is_available():
    raise RuntimeError("ThinWedge PyTorch runtime requires CUDA")
```

Recommended startup logging:

- `torch.__version__`
- `torch.version.cuda`
- `torch.cuda.get_device_name(0)`

## Training Contract

Generated training code should:

- choose a specific device with `torch.device("cuda")`
- use `DataLoader(..., pin_memory=True)`
- use `persistent_workers=True` when loaders are long-lived
- move tensors with `non_blocking=True`
- use AMP for CUDA training
- save resumable checkpoints frequently

Preferred training structure:

```python
def build_model(config): ...
def build_dataloaders(config): ...
def train_epoch(model, loader, optimizer, scaler, device): ...
def evaluate(model, loader, device): ...
def save_checkpoint(path, model, optimizer, epoch, step, metrics): ...
```

Recommended AMP pattern:

```python
scaler = torch.amp.GradScaler("cuda")

with torch.autocast(device_type="cuda", dtype=torch.float16):
    loss = model_loss(...)
```

## Checkpoint Contract

Checkpoints should be dictionaries containing at least:

- `model_state_dict`
- `optimizer_state_dict`
- `epoch`
- `step`
- `loss`
- `metrics`
- `config`

Preferred locations:

- `/workspace/thinwedge/checkpoints/<jobId>/last.pt`
- `/workspace/thinwedge/checkpoints/<jobId>/best.pt`

## Inference Contract

Generated inference code must:

- load the model once during process startup
- call `model.eval()`
- use `torch.inference_mode()`
- avoid per-request model initialization

Preferred function shape:

```python
def predict_batch(model, batch, device):
    with torch.inference_mode():
        return model(batch.to(device, non_blocking=True))
```

## HTTP Statistical-Model API Contract

If ThinWedge generates a statistical-model API server, it should:

- use FastAPI
- serve on `0.0.0.0:8000`
- run one worker process per GPU Pod
- expose `GET /healthz`
- expose `GET /readyz`
- expose one authenticated prediction endpoint

Because Runpod’s public proxy has a request timeout, the service should be used only for short inference calls.

## Offline Batch Inference Contract

For long batch work, generated code should expose a runner like:

```python
def run_offline_inference(
    input_manifest_path: str,
    output_root: str,
    model_path: str,
    batch_size: int,
    shard_index: int | None = None,
    shard_count: int | None = None,
) -> dict:
    ...
```

The runner should:

- read inputs from `/workspace`
- write incremental outputs to `/workspace`
- avoid holding the full job result in memory
- emit a machine-readable summary manifest

## Multi-GPU Rules

If ThinWedge requests multi-GPU execution, generated code should:

- use one process per GPU
- prefer `DistributedDataParallel`
- use the `nccl` backend

Generated code should not use `DataParallel` as the default.

## Forbidden Patterns

Do not generate:

- multiple Uvicorn workers for one-GPU inference Pods
- request handlers that execute arbitrary shell commands
- hidden filesystem writes outside `/workspace`
- synchronous HTTP handlers for long jobs
- runtime model downloads on first request when the model can be preloaded or cached

## Filesystem Layout

Use:

- `/workspace/thinwedge/model-repo`
- `/workspace/thinwedge/models`
- `/workspace/thinwedge/datasets`
- `/workspace/thinwedge/checkpoints`
- `/workspace/thinwedge/outputs/<jobId>`
- `/workspace/thinwedge/logs/<jobId>`

## ThinWedge Prompting Guidance

When ThinWedge asks an LLM to generate GPU PyTorch code, the prompt should require:

- CUDA-only runtime checks
- explicit training/inference entrypoints
- AMP usage
- checkpointing
- one-worker API behavior
- `/workspace`-only output paths
- no runtime dependency installation

## Sources

- Runpod custom Pod templates: https://docs.runpod.io/pods/templates/create-custom-template
- Runpod expose ports: https://docs.runpod.io/pods/configuration/expose-ports
- Runpod environment variables: https://docs.runpod.io/pods/templates/environment-variables
- Runpod storage options: https://docs.runpod.io/pods/storage/types
- PyTorch local install docs: https://docs.pytorch.org/get-started/locally/
- PyTorch CUDA notes: https://docs.pytorch.org/docs/2.9/notes/cuda.html
- PyTorch `DistributedDataParallel`: https://docs.pytorch.org/docs/stable/generated/torch.nn.parallel.DistributedDataParallel.html
- PyTorch `DataParallel`: https://docs.pytorch.org/docs/stable/generated/torch.nn.DataParallel.html
- PyTorch DataLoader docs: https://docs.pytorch.org/docs/stable/data.html
- PyTorch AMP examples: https://docs.pytorch.org/docs/stable/notes/amp_examples.html
- PyTorch saving/loading models: https://docs.pytorch.org/tutorials/beginner/saving_loading_models.html
