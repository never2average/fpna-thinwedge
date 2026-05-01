# ThinWedge CLI (Rust Implementation)

We provide ThinWedge as a standalone executable to ensure a zero-dependency install.

## Installing ThinWedge

Install the published ThinWedge package for your environment, then run:

```shell
thinwedge
```

See [`docs/install.md`](../docs/install.md) for environment-specific installation details.

## Documentation quickstart

- First run with ThinWedge? Start with [`docs/getting-started.md`](../docs/getting-started.md) (links to the walkthrough for prompts, keyboard shortcuts, and session management).
- Want deeper control? See [`docs/config.md`](../docs/config.md) and [`docs/install.md`](../docs/install.md).

## What's new in the Rust CLI

The Rust implementation is now the maintained ThinWedge CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

ThinWedge supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`docs/config.md`](../docs/config.md) for details.

### ThinWedge ML Tool Data

ThinWedge's `statisticalmodels.*` and `trainingenvironments.*` tool state is local-filesystem based in this build. The TUI reads and writes JSON under `THINWEDGE_HOME/thinwedge/ml/`, including `statisticalmodels.json`, `trainingenvironments.json`, `jobs/*.json`, `evals/*.json`, runtime context files under `runtime/*.json`, and environment session caches under `environments/*.json`. ThinWedge also exposes direct API-backed cost namespaces for the built-in roles:

- `llmcosts.*` for LLM market context from Artificial Analysis
- `infracosts.*` for AWS infrastructure pricing context

Model and environment records can declare executable actions directly in those JSON files. ThinWedge looks for model bindings like `tools.submitTraining` and `tools.submitBatchInference`, and environment bindings like `tools.launch`, `tools.attach`, and `tools.stop`. Each binding supplies a local shell command plus optional working directory and environment variables.

For model-driven execution, records can also declare an `inference` block and a `repository` block. ThinWedge passes those through to commands as environment variables such as `THINWEDGE_INFERENCE_PROVIDER`, `THINWEDGE_INFERENCE_MODEL`, `THINWEDGE_INFERENCE_BASE_URL`, `THINWEDGE_INFERENCE_API_KEY_ENV`, and `THINWEDGE_MODEL_REPOSITORY_ROOT`.

The current migration target uses this terminology:

- `inference` = statistical-model API calls
- `LLM inference` = the separate OpenRouter path

And this backend split:

- `trainingenvironments.launch` / `attach` / `stop` use Runpod Pods as the environment source of truth
- `statisticalmodels.submitJob(type="training")` runs against a live Runpod environment
- `statisticalmodels.submitJob(type="batchInference")` runs statistical-model inference against a live Runpod environment
- OpenRouter remains a separate LLM-only path
- `llmcosts.listModels` / `getModel` / `compareModels` query Artificial Analysis for LLM market pricing, speed, and benchmark context
- `infracosts.describeAwsServices` / `searchAwsServices` / `searchAwsPriceListAttributeNames` / `searchAwsPriceListAttributeValues` / `getAwsProducts` / `getAwsVmPrice` / `estimateAwsBoq` use the AWS Price List API for BOQ and list-price infrastructure estimation plus filter discovery
- `infracosts.getAwsCostAndUsage` / `getAwsDimensionValues` / `getAwsCostForecast` / `getAwsAnomalies` use AWS Cost Explorer for actual billing analysis
- `infracosts.queryAwsByService` / `queryAwsByAccount` provide common billing-query shortcuts on top of Cost Explorer
- `infracosts.listBillingViews` uses the AWS Billing API for scoped billing-view discovery

#### Cost-context API environment

ThinWedge expects these credentials when `llmcosts.*` and `infracosts.*` tools are used:

- `ARTIFICIAL_ANALYSIS_API_KEY` for the Artificial Analysis free data API
- AWS credentials from the standard AWS SDK chain for AWS infrastructure pricing

Optional AWS tool arguments:

- `profile`: AWS profile name to use for SigV4 signing
- `apiRegion`: AWS API region, defaulting to `us-east-1` for Price List, Cost Explorer, and Billing

#### `metadata.runpod` contract

Runpod environment configuration lives under `trainingenvironments.json -> environments[].metadata.runpod`.

Required v1 fields:

- `gpuCount`
- `volumeMountPath`
- `workspacePath`
- `exposedHttpPort`
- `supportsSsh`

One of these must also be provided:

- `templateId`
- `imageName`

Optional v1 fields:

- `name`
- `imageName`
- `cloudType`
- `gpuTypeId`
- `containerDiskInGb`
- `volumeInGb`
- `networkVolumeId`
- `dataCenterIds`
- `supportPublicIp`
- `sshPrivateKeyPath`
- `dockerArgs`
- `env`
- `stopMode`
- `startupTimeoutSec`

#### Session cache contract

Runpod session state is cached at `THINWEDGE_HOME/thinwedge/ml/environments/<environmentId>.json`.

Those files are non-authoritative mirrors of the remote Pod state and should contain:

- `environmentId`
- `provider`
- `podId`
- `templateId`
- `podName`
- `status`
- `desiredStatus`
- `workspacePath`
- `volumeMountPath`
- `publicIp`
- `httpEndpoint`
- `portMappings`
- `supportsSsh`
- `attach`
- `lastRemoteSyncAt`
- `launchDisposition`
- `stopMode`
- `contextPath`
- `rawPod`

The checked-in ThinWedge ML command package lives in [`scripts/thinwedge-ml/`](./scripts/thinwedge-ml):

- `common.sh`: generic filesystem, JSON, and local artifact helpers
- `runpod.sh`: shared Runpod contract, API, session, and payload-validation helpers
- `train.sh`: training command implementation
- `batch_infer.sh`: batch inference command implementation
- `launch.sh`: environment launch/start implementation
- `attach.sh`: environment attach/inspect implementation
- `stop.sh`: environment stop/terminate implementation
- `seed-fixtures.sh`: creates sample model and environment registries plus wrapper entrypoints in a temp workspace. The seeded model registry no longer treats OpenRouter as statistical-model inference; it seeds Runpod-oriented repository and batch-entrypoint metadata instead.
- `smoke-test.sh`: shell-only Runpod verification entrypoint. Run `scripts/thinwedge-ml/smoke-test.sh` for offline mock validation of lifecycle, training, batch inference, and negative payload checks. Run `scripts/thinwedge-ml/smoke-test.sh --live-runpod` for an opt-in live Runpod lifecycle smoke path when `RUNPOD_API_KEY` and either a real template or an image-only Pod configuration are available.

Live Runpod notes:

- Image-only Pod creation is supported when `imageName` is supplied and `templateId` is omitted.
- For full SSH paths, ThinWedge can carry `sshPrivateKeyPath` in `metadata.runpod` and will pass that identity to `ssh`/`scp` automatically.
- The seeded fixture environment exports both `SSH_PUBLIC_KEY` and `PUBLIC_KEY` so official Runpod images and custom SSH startup flows can both consume the same key material.

The checked-in ThinWedge ML design contracts live in [`docs/`](./docs):

- [`runpod-template-spec.md`](./docs/runpod-template-spec.md): pinned Pod template families, filesystem layout, and `metadata.runpod` conventions
- [`thinwedge-cuml-codegen-spec.md`](./docs/thinwedge-cuml-codegen-spec.md): code-generation rules for RAPIDS / cuML workloads
- [`thinwedge-pytorch-codegen-spec.md`](./docs/thinwedge-pytorch-codegen-spec.md): code-generation rules for GPU PyTorch workloads
- [`thinwedge-batch-infer-contract.md`](./docs/thinwedge-batch-infer-contract.md): target Phase 5 execution contract for `batch_infer.sh`

Tool-to-command mapping in the seeded fixture set:

- `statisticalmodels.submitJob` with `type: "training"` -> `bash <repo>/scripts/thinwedge-ml/train.sh`
- `statisticalmodels.submitJob` with `type: "batchInference"` -> `bash <repo>/scripts/thinwedge-ml/batch_infer.sh`
- `trainingenvironments.launch` -> `bash <repo>/scripts/thinwedge-ml/launch.sh`
- `trainingenvironments.attach` -> `bash <repo>/scripts/thinwedge-ml/attach.sh`
- `trainingenvironments.stop` -> `bash <repo>/scripts/thinwedge-ml/stop.sh`

First-party cost tools exposed directly by ThinWedge:

- `llmcosts.listModels`: list LLM cost/speed market context from Artificial Analysis
- `llmcosts.getModel`: inspect one Artificial Analysis LLM entry by `modelId`, `slug`, or `name`
- `llmcosts.compareModels`: compare multiple models across price, latency, speed, intelligence index, and coding index
- `infracosts.describeAwsServices`: query AWS Price List service metadata
- `infracosts.searchAwsServices`: fuzzy-search AWS Price List service codes before building price queries
- `infracosts.searchAwsPriceListAttributeNames`: discover valid attribute names for one Price List service
- `infracosts.searchAwsPriceListAttributeValues`: sample product pages to discover candidate values for one Price List attribute field
- `infracosts.getAwsCostAndUsage`: query AWS Cost Explorer actual cost and usage data with optional billing views, filters, and groupings
- `infracosts.getAwsDimensionValues`: query AWS Cost Explorer dimension values for accounts, services, regions, usage types, and similar billing filters
- `infracosts.getAwsCostForecast`: query AWS Cost Explorer forecasted spend from billing history
- `infracosts.getAwsAnomalies`: query AWS Cost Explorer anomaly detection results
- `infracosts.queryAwsByService`: shortcut actual-billing query filtered to one AWS service
- `infracosts.queryAwsByAccount`: shortcut actual-billing query filtered to one or more linked accounts
- `infracosts.listBillingViews`: list AWS billing views for scoped FP&A access
- `infracosts.getAwsProducts`: query AWS Price List products with explicit filters
- `infracosts.getAwsVmPrice`: convenience path for EC2 VM price context using `instanceType` plus optional region/location and OS filters
- `infracosts.estimateAwsBoq`: estimate a multi-line BOQ using Price List filters, quantities, and unit selection

The `train.sh` payload contract is:

- Required environment:
  - an active Runpod session cache at `THINWEDGE_HOME/thinwedge/ml/environments/<environmentId>.json`
  - a valid `metadata.runpod.env.THINWEDGE_MODEL_REPOSITORY_ROOT` remote repository root
- Required payload fields:
  - none beyond valid JSON for the base training job
- Optional payload fields:
  - `epochs`
  - `learningRate`
  - `dataset`
  - `codegen.files`
- Each `codegen.files` entry supports:
  - `path`: repository-relative output path inside the remote model repository
  - `instruction`: prompt or build instruction for the generated implementation
  - `language`: optional language hint
- `train.sh` stages the payload into the remote Runpod workspace, invokes the remote repository entrypoint, and expects the Pod-side process to write:
  - a training artifact manifest
  - an eval manifest
  - a generated-files manifest when `codegen.files` is present
- ThinWedge only mirrors thin local indexes after remote success. There is no local-authoritative fallback for training or code generation in this path.

`train.sh` emits this JSON summary on stdout:

- `tool`
- `jobId`
- `jobType`
- `modelId`
- `environmentId`
- `provider`
- `podId`
- `workspacePath`
- `remoteCommand`
- `artifactManifestPath`
- `evalManifestPath`
- `generatedFiles`
- `status`

Runpod execution modes:

- Mock mode: set `THINWEDGE_RUNPOD_MOCK_DIR` and provide canned Pod responses. The scripts keep using the same Runpod contracts, but remote uploads, downloads, and execution are mirrored under `<mockDir>/uploaded/`.
- Live mode: unset `THINWEDGE_RUNPOD_MOCK_DIR`, export `RUNPOD_API_KEY`, and point the environment record at a real Pod template/image whose remote workspace contains the repository entrypoints ThinWedge invokes.

Example model record:

```json
{
  "id": "model-pricing",
  "visibleToRoles": ["pricing_researcher"],
  "defaultEnvironmentId": "env-pricing",
  "inference": {
    "providerId": "openrouter",
    "modelName": "thinwedge/gpt-4.1-mini",
    "baseUrl": "https://openrouter.ai/api/v1",
    "apiKeyEnv": "OPENROUTER_API_KEY",
    "wireApi": "chatCompletions"
  },
  "repository": {
    "root": "/abs/path/to/model-pricing-repo",
    "configPath": "/abs/path/to/model-pricing-repo/config.yaml",
    "refName": "main",
    "entrypoint": "scripts/train.sh",
    "batchEntryPoint": "scripts/run_batch_inference.py"
  },
  "tools": {
    "submitTraining": {
      "command": "bash /abs/path/to/thinwedge-rs/scripts/thinwedge-ml/train.sh",
      "workingDirectory": "/abs/path/to/model-pricing-repo"
    },
    "submitBatchInference": {
      "command": "bash /abs/path/to/thinwedge-rs/scripts/thinwedge-ml/batch_infer.sh",
      "workingDirectory": "/abs/path/to/model-pricing-repo"
    }
  }
}
```

Example training payload with code generation:

```json
{
  "type": "training",
  "modelId": "model-pricing",
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
```

Example environment record:

```json
{
  "id": "env-pricing",
  "visibleToRoles": ["pricing_researcher"],
  "repository": {
    "root": "/abs/path/to/env-pricing-repo",
    "refName": "gpu-main",
    "entrypoint": "ops/launch.sh"
  },
  "metadata": {
    "provider": "runpod",
    "runpod": {
      "gpuCount": 1,
      "gpuTypeId": "NVIDIA A100 80GB PCIe",
      "imageName": "runpod/pytorch:2.1.0-py3.10-cuda11.8.0-devel-ubuntu22.04",
      "volumeMountPath": "/workspace",
      "workspacePath": "/workspace/thinwedge/env-pricing",
      "exposedHttpPort": 8000,
      "supportsSsh": true,
      "name": "thinwedge-pricing-env",
      "cloudType": "COMMUNITY",
      "containerDiskInGb": 50,
      "volumeInGb": 100,
      "supportPublicIp": true,
      "sshPrivateKeyPath": "/abs/path/to/id_ed25519",
      "stopMode": "stop",
      "startupTimeoutSec": 900
    }
  },
  "tools": {
    "launch": {
      "command": "bash /abs/path/to/thinwedge-rs/scripts/thinwedge-ml/launch.sh",
      "workingDirectory": "/abs/path/to/env-pricing-repo"
    },
    "attach": {
      "command": "bash /abs/path/to/thinwedge-rs/scripts/thinwedge-ml/attach.sh",
      "workingDirectory": "/abs/path/to/env-pricing-repo"
    },
    "stop": {
      "command": "bash /abs/path/to/thinwedge-rs/scripts/thinwedge-ml/stop.sh",
      "workingDirectory": "/abs/path/to/env-pricing-repo"
    }
  }
}
```

### Model Context Protocol Support

#### MCP client

ThinWedge functions as an MCP client that allows the ThinWedge CLI and IDE extension to connect to MCP servers on startup. See the [`configuration documentation`](../docs/config.md#connecting-to-mcp-servers) for details.

#### MCP server (experimental)

ThinWedge can be launched as an MCP _server_ by running `thinwedge mcp-server`. This allows _other_ MCP clients to use ThinWedge as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector thinwedge mcp-server
```

Use `thinwedge mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `thinwedge mcp-server` to run the MCP server directly.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS. When ThinWedge detects that it is running under WSL 2 inside Windows Terminal (`WT_SESSION` is set), the TUI automatically falls back to native Windows toast notifications so approval prompts and completed turns surface even though Windows Terminal does not implement OSC 9.

### `thinwedge exec` to run ThinWedge programmatically/non-interactively

To run ThinWedge non-interactively, run `thinwedge exec PROMPT` (you can also pass the prompt via `stdin`) and ThinWedge will work on your task until it decides that it is done and exits. If you provide both a prompt argument and piped stdin, ThinWedge appends stdin as a `<stdin>` block after the prompt so patterns like `echo "my output" | thinwedge exec "Summarize this concisely"` work naturally. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.
Use `thinwedge exec --ephemeral ...` to run without persisting session rollout files to disk.

### Experimenting with the ThinWedge Sandbox

To test what happens when a command is run under the sandbox provided by ThinWedge, use the following subcommands:

```
# macOS
thinwedge sandbox macos [--log-denials] [COMMAND]...

# Linux
thinwedge sandbox linux [COMMAND]...

# Windows
thinwedge sandbox windows [COMMAND]...

# Legacy aliases
thinwedge debug seatbelt [--log-denials] [COMMAND]...
thinwedge debug landlock [COMMAND]...
```

To try a writable legacy sandbox mode with these commands, pass an explicit config override such
as `-c 'sandbox_mode="workspace-write"'`.

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run ThinWedge with the default, read-only sandbox
thinwedge --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
thinwedge --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
thinwedge --sandbox danger-full-access
```

The same setting can be persisted in `~/.thinwedge/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.
In `workspace-write`, ThinWedge also includes `~/.thinwedge/memories` in its writable roots so memory maintenance does not require an extra approval.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for ThinWedge. Ultimately, we hope this becomes a library crate that is generally useful for building other Rust/native applications that use ThinWedge.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.

If you want to contribute or inspect behavior in detail, start by reading the module-level `README.md` files under each crate and run the project workspace from the top-level `thinwedge-rs` directory so shared config, features, and build scripts stay aligned.
