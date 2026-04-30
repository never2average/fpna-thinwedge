# ThinWedge `batch_infer.sh` Execution Contract

Last reviewed: April 30, 2026

This document defines the target contract for the Phase 5 rewrite of `scripts/thinwedge-ml/batch_infer.sh`.

## Goal

Make `batch_infer.sh` run statistical-model inference against a live Runpod Pod.

This path is not LLM inference.

Session terminology:

- `inference` = statistical-model API call
- `LLM inference` = OpenRouter path

## Preconditions

`batch_infer.sh` must require:

- `THINWEDGE_MODEL_ID`
- `THINWEDGE_ACTION`
- `THINWEDGE_AGENT_ROLE`
- `THINWEDGE_JOB_ID`
- `THINWEDGE_ENVIRONMENT_ID`
- a verified model repository summary
- a live Runpod session cache with `podId`
- a valid `metadata.runpod`

## Payload Contract

Accepted payload fields:

- `inputPath`
- `inputUri`
- `outputPath`
- `shardIndex`
- `shardCount`
- `batchSize`
- `timeoutSec`
- `mode`

Rules:

- at least one of `inputPath` or `inputUri` is required
- `outputPath` is required
- `mode` may be omitted

Normalized v1 payload shape:

```json
{
  "inputPath": "/workspace/thinwedge/datasets/pricing.parquet",
  "inputUri": null,
  "outputPath": "/workspace/thinwedge/outputs/job-123/predictions.parquet",
  "shardIndex": 0,
  "shardCount": 8,
  "batchSize": 4096,
  "timeoutSec": 1800,
  "mode": "http",
  "raw": {}
}
```

## Execution Modes

### Preferred mode: HTTP

Use when:

- the model Pod exposes a short-running statistical-model API
- each inference request fits inside the public proxy timeout

Behavior:

- derive `httpEndpoint` from the live session
- call the Pod API
- write remote outputs and manifests under `/workspace`
- mirror thin local indexes after remote success

The service must bind to `0.0.0.0:8000`.

### Fallback mode: SSH one-shot worker

Use when:

- the batch job is too large for the proxy timeout
- the workload needs direct file-based execution
- the Pod API is unavailable

Behavior:

- stage the normalized payload into `/workspace/thinwedge/jobs/<jobId>/batch-payload.json`
- SSH into the Pod
- invoke a remote batch runner
- require the remote batch runner to use GPU pandas

Required remote command pattern:

```bash
python -m cudf.pandas /workspace/thinwedge/model-repo/scripts/run_batch_inference.py \
  --thinwedge-payload /workspace/thinwedge/jobs/<jobId>/batch-payload.json \
  --thinwedge-output-dir /workspace/thinwedge/jobs/<jobId>
```

ThinWedge may choose a more specific remote path, but the worker must run under `python -m cudf.pandas` for RAPIDS-oriented batch inference.

## Remote Workspace Contract

For job `job-123`, `batch_infer.sh` should use:

- payload: `/workspace/thinwedge/jobs/job-123/batch-payload.json`
- stdout log: `/workspace/thinwedge/jobs/job-123/batch.stdout.log`
- stderr log: `/workspace/thinwedge/jobs/job-123/batch.stderr.log`
- artifact manifest: `/workspace/thinwedge/jobs/job-123/batch-artifact-manifest.json`
- eval manifest: `/workspace/thinwedge/jobs/job-123/batch-eval-manifest.json`

The output dataset itself belongs at the caller-specified `outputPath`.

## Local Mirrored Indexes

After remote success, ThinWedge should mirror only thin local indexes:

- `CODEX_HOME/thinwedge/ml/artifacts/batch-inference/<jobId>.json`
- `CODEX_HOME/thinwedge/ml/evals/eval-<jobId>.json`

Those local files are indexes, not the source of truth.

## Stdout Contract

`batch_infer.sh` should emit one JSON object on stdout with:

- `tool`
- `jobId`
- `jobType`
- `modelId`
- `environmentId`
- `provider`
- `podId`
- `httpEndpoint`
- `executionMode`
- `remoteCommand`
- `inputPath`
- `inputUri`
- `outputPath`
- `artifactManifestPath`
- `evalManifestPath`
- `shard`
- `status`

Example:

```json
{
  "tool": "statisticalmodels.submitJob",
  "jobId": "job-123",
  "jobType": "batchInference",
  "modelId": "model-pricing",
  "environmentId": "env-pricing",
  "provider": "runpod",
  "podId": "pod-abc",
  "httpEndpoint": "https://pod-abc-8000.proxy.runpod.net",
  "executionMode": "ssh",
  "remoteCommand": "python -m cudf.pandas /workspace/thinwedge/model-repo/scripts/run_batch_inference.py ...",
  "inputPath": "/workspace/thinwedge/datasets/pricing.parquet",
  "inputUri": null,
  "outputPath": "/workspace/thinwedge/outputs/job-123/predictions.parquet",
  "artifactManifestPath": "/workspace/thinwedge/jobs/job-123/batch-artifact-manifest.json",
  "evalManifestPath": "/workspace/thinwedge/jobs/job-123/batch-eval-manifest.json",
  "shard": {
    "index": 0,
    "count": 8
  },
  "status": "completed"
}
```

## Manifest Expectations

The remote artifact manifest should include:

- `jobId`
- `modelId`
- `environmentId`
- `provider`
- `podId`
- `inputPath`
- `inputUri`
- `outputPath`
- `rowCount` when known
- `executionMode`
- `generatedAt`

The remote eval manifest should include:

- `id`
- `jobId`
- `modelId`
- `status`
- `summary`
- `metrics`
- `artifactPaths`
- `createdAt`

## Error Handling

`batch_infer.sh` must fail hard when:

- there is no live Runpod session
- neither `inputPath` nor `inputUri` is supplied
- `outputPath` is missing
- the Pod API call fails
- the SSH worker fails
- remote manifests cannot be retrieved

There should be no OpenRouter fallback in this path.

## Sources

- Runpod Pods overview: https://docs.runpod.io/pods/overview
- Runpod API overview: https://docs.runpod.io/api-reference/overview
- Runpod SSH: https://docs.runpod.io/pods/configuration/use-ssh
- Runpod expose ports: https://docs.runpod.io/pods/configuration/expose-ports
- `cudf.pandas` usage: https://docs.rapids.ai/api/cudf/stable/cudf_pandas/usage/
