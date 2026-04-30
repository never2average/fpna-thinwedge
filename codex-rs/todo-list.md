# ThinWedge Runpod Migration Todo List

This file is the implementation tracker for the Runpod-backed ThinWedge ML migration.

Working rules for this pass:
- Do not run `cargo build`, `cargo check`, `cargo test`, or workspace-wide Rust builds until all phases below are implemented.
- Use shell-only validation while implementation is in progress.
- `inference` means statistical-model API calls.
- `LLM inference` means the separate OpenRouter path.

## Phase 0: Tracker Setup
- [x] Create this `todo-list.md`
- [x] Keep this file updated as phases complete
- [x] Record any scope changes here before implementing them

## Phase 1: Schema And Contracts
### Environment Runpod config
- [x] Define `metadata.runpod` contract for `trainingenvironments.json`
- [x] Decide required fields for v1:
  - [x] `templateId` or `imageName`
  - [x] `gpuCount`
  - [x] `volumeMountPath`
  - [x] `workspacePath`
  - [x] `exposedHttpPort`
  - [x] `supportsSsh`
- [x] Decide optional fields for v1:
  - [x] `name`
  - [x] `imageName`
  - [x] `cloudType`
  - [x] `gpuTypeId`
  - [x] `containerDiskInGb`
  - [x] `volumeInGb`
  - [x] `networkVolumeId`
  - [x] `dataCenterIds`
  - [x] `supportPublicIp`
  - [x] `dockerArgs`
  - [x] `env`
  - [x] `stopMode`
  - [x] `startupTimeoutSec`

### Session cache contract
- [x] Define `CODEX_HOME/thinwedge/ml/environments/<environmentId>.json` session schema
- [x] Include:
  - [x] `environmentId`
  - [x] `provider`
  - [x] `podId`
  - [x] `templateId`
  - [x] `podName`
  - [x] `status`
  - [x] `desiredStatus`
  - [x] `workspacePath`
  - [x] `volumeMountPath`
  - [x] `publicIp`
  - [x] `httpEndpoint`
  - [x] `portMappings`
  - [x] `supportsSsh`
  - [x] `attach`
  - [x] `lastRemoteSyncAt`
  - [x] `launchDisposition`
  - [x] `stopMode`
  - [x] `contextPath`
  - [x] `rawPod`

### Script stdout contract
- [x] Define stdout JSON contract for `launch.sh`
- [x] Define stdout JSON contract for `attach.sh`
- [x] Define stdout JSON contract for `stop.sh`
- [x] Define stdout JSON contract for `train.sh`
  - [x] `tool`
  - [x] `jobId`
  - [x] `jobType`
  - [x] `modelId`
  - [x] `environmentId`
  - [x] `provider`
  - [x] `podId`
  - [x] `workspacePath`
  - [x] `remoteCommand`
  - [x] `artifactManifestPath`
  - [x] `evalManifestPath`
  - [x] `generatedFiles`
  - [x] `status`
- [x] Define stdout JSON contract for `batch_infer.sh`
  - [x] `tool`
  - [x] `jobId`
  - [x] `jobType`
  - [x] `modelId`
  - [x] `environmentId`
  - [x] `provider`
  - [x] `podId`
  - [x] `httpEndpoint`
  - [x] `executionMode`
  - [x] `remoteCommand`
  - [x] `inputPath`
  - [x] `inputUri`
  - [x] `outputPath`
  - [x] `artifactManifestPath`
  - [x] `evalManifestPath`
  - [x] `shard`
  - [x] `status`

## Phase 2: Shared Runpod Helper Layer
### Preserve generic helpers
- [x] Keep reusable filesystem / JSON helpers in `common.sh`
- [x] Remove statistical-model inference assumptions from shared helpers

### Add Runpod helpers
- [x] Add `thinwedge_runpod_api_base`
- [x] Add `thinwedge_require_runpod_api_key`
- [x] Add `thinwedge_runpod_get_pod`
- [x] Add `thinwedge_runpod_create_pod`
- [x] Add `thinwedge_runpod_start_pod`
- [x] Add `thinwedge_runpod_stop_pod`
- [x] Add `thinwedge_runpod_delete_pod`
- [x] Add `thinwedge_runpod_poll_pod_status`
- [x] Add `thinwedge_runpod_build_http_endpoint`
- [x] Add `thinwedge_runpod_extract_port_mapping`
- [x] Add `thinwedge_runpod_session_path`
- [x] Add `thinwedge_runpod_read_session`
- [x] Add `thinwedge_runpod_write_session`
- [x] Add `thinwedge_runpod_remote_exec_ssh`
- [x] Add `thinwedge_runpod_upload_payload`
- [x] Add `thinwedge_runpod_download_manifest`

### Validation helpers
- [x] Add helper to read and validate `metadata.runpod`
- [x] Add helper to validate training payloads
- [x] Add helper to validate batch inference payloads

## Phase 3: Environment Commands
### `launch.sh`
- [x] Rework `launch.sh` to read `metadata.runpod`
- [x] Reuse cached `podId` when valid
- [x] Start stopped Pods
- [x] Recreate terminated/missing Pods
- [x] Create new Pod when no valid session exists
- [x] Poll until Pod is running
- [x] Derive `httpEndpoint`
- [x] Derive SSH / attach metadata
- [x] Persist session cache
- [x] Emit stdout JSON contract

### `attach.sh`
- [x] Rework `attach.sh` to query live Pod state
- [x] Refresh session cache from Runpod
- [x] Return HTTP endpoint details
- [x] Return SSH / IDE attach details when available
- [x] Emit stdout JSON contract

### `stop.sh`
- [x] Rework `stop.sh` to call Runpod stop API
- [x] Support optional terminate mode
- [x] Poll until remote status is settled
- [x] Update local session cache only after remote success
- [x] Emit stdout JSON contract

## Phase 4: Training Command
### `train.sh`
- [x] Remove local-authoritative training behavior
- [x] Require a live Runpod-backed environment session
- [x] Validate Pod/template/volume/workspace assumptions
- [x] Stage training payload into remote workspace
- [x] Invoke remote training entrypoint
- [x] Keep code generation inside the Pod / remote workspace
- [x] Write remote training artifact manifest
- [x] Write remote eval manifest
- [x] Write remote generated-files manifest when applicable
- [x] Mirror only thin local indexes if needed for TUI compatibility
- [x] Emit stdout JSON contract
- [x] Fail hard on remote training/codegen failure

## Phase 5: Batch Inference Command
### `batch_infer.sh`
- [x] Remove OpenRouter/statistical-model confusion from batch inference
- [x] Require a live Runpod-backed environment session
- [x] Validate payload fields:
  - [x] `inputPath` or `inputUri`
  - [x] `outputPath`
  - [x] optional `shardIndex`
  - [x] optional `shardCount`
  - [x] optional `batchSize`
  - [x] optional `timeoutSec`
- [x] Preferred path: call the trained model HTTP API on the Pod
- [x] Fallback path: remote one-shot worker via SSH
- [x] Ensure remote one-shot worker uses `python -m cudf.pandas ...`
- [x] Write outputs to remote workspace or network volume
- [x] Write remote artifact/eval manifests
- [x] Mirror only thin local indexes if needed
- [x] Emit stdout JSON contract

## Phase 6: Rust Source Alignment
### `types.rs`
- [x] Add typed Runpod support for environment metadata
- [x] Add `RunpodEnvironmentConfig`
- [x] Add stop mode type if needed
- [x] Decide whether session summaries stay shell-owned or become typed
- [x] Consider extending environment status beyond `running|stopped`

### `spec.rs`
- [x] Update environment command descriptions away from “local state only”
- [x] Ensure training and batch inference are treated as environment-backed execution
- [x] Keep local jobs/evals as indexes while remote storage is source of truth
- [x] Capture script stdout as remote execution summary

### Other source alignment
- [x] Remove stale OpenRouter assumptions from statistical-model batch inference
- [x] Preserve OpenRouter only for separate LLM codegen paths

## Phase 7: Fixtures, Docs, And Shell Testing
### Fixtures
- [x] Rework `seed-fixtures.sh` around Runpod-oriented config
- [x] Add mock mode for offline shell validation
- [x] Remove local-only environment semantics from fixtures

### Docs
- [x] Update `README.md` with `metadata.runpod` schema
- [x] Document session cache schema
- [x] Document training payload contract
- [x] Document batch inference payload contract
- [x] Document `inference` vs `LLM inference`
- [x] Document mock mode vs live Runpod mode
- [x] Add checked-in Runpod template and codegen design contracts under `docs/`

### Shell-only verification
- [x] Keep `bash -n` coverage for all scripts
- [x] Add mock Runpod API tests for Pod create/get/start/stop flows
- [x] Add session cache verification tests
- [x] Add mock training remote execution test
- [x] Add mock batch inference remote execution test
- [x] Add payload validation failure tests
- [x] Add opt-in live Runpod smoke path later

## Deferred Until All Phases Are Done
- [x] Run Rust formatter (`cargo fmt` fallback because `just` is unavailable on this toolchain)
- [ ] Run Rust crate checks/tests
- [ ] Run snapshot/schema regeneration if needed
- [x] Do final integrated verification

## Notes / Decisions
- [x] Decide whether `templateId` is mandatory in v1 or whether direct image config is allowed
- [x] Decide whether `batch_infer.sh` supports only HTTP API mode in v1 or SSH fallback too
- [x] Decide whether `train.sh` retains any LLM codegen capability in v1 or only remote execution
- [x] Decide whether local `evals/*.json` remain mandatory or optional indexes

### Current decisions
- [x] `train.sh` now uses remote execution only. Any code generation request is staged into the Runpod job payload and is expected to happen inside the Pod / remote workspace, not in the local checkout.
- [x] The shell smoke path now uses Runpod mock mode for lifecycle + training verification until the Phase 5 batch inference rewrite lands.
- [x] ThinWedge now has checked-in design contracts for Runpod templates, cuML codegen, GPU PyTorch codegen, and the target `batch_infer.sh` execution contract.
- [x] `batch_infer.sh` now prefers the Pod HTTP API and falls back to a `python -m cudf.pandas ...` SSH worker when HTTP is unavailable or bypassed.
- [x] Rust-side execution records now capture parsed JSON stdout summaries when the shell script emits one, and environment metadata is typed for `metadata.runpod`.
- [x] Direct image-only creation is now supported for live Runpod Pods when `imageName` is supplied and `templateId` is omitted.
- [x] Live Runpod validation now covers a full official PyTorch Pod path: `launch -> train -> batchInference -> stop`.
- [x] Local `evals/*.json` remain mandatory as thin indexes even though remote manifests are the source of truth.
- [x] `just fmt` could not be installed on the pinned Rust/Cargo toolchain here, so `cargo fmt` was used as the formatter fallback.

## Phase 8: Cost Context APIs
### ThinWedge role-facing cost tools
- [x] Split first-party cost tools into `llmcosts.*` and `infracosts.*`
- [x] Keep Artificial Analysis scoped to LLM market pricing and speed context
- [x] Keep AWS scoped to non-LLM infrastructure pricing such as VMs and storage
- [x] Add `aws_cost_engineer` as a built-in specialist role for AWS BOQ work

### Artificial Analysis
- [x] Add `llmcosts.listModels`
- [x] Add `llmcosts.getModel`
- [x] Add `llmcosts.compareModels`
- [x] Use the Artificial Analysis free data API with `ARTIFICIAL_ANALYSIS_API_KEY`

### AWS infrastructure pricing
- [x] Add `infracosts.describeAwsServices`
- [x] Add `infracosts.searchAwsServices`
- [x] Add `infracosts.searchAwsPriceListAttributeNames`
- [x] Add `infracosts.searchAwsPriceListAttributeValues`
- [x] Add `infracosts.getAwsCostAndUsage`
- [x] Add `infracosts.getAwsDimensionValues`
- [x] Add `infracosts.getAwsCostForecast`
- [x] Add `infracosts.getAwsAnomalies`
- [x] Add `infracosts.queryAwsByService`
- [x] Add `infracosts.queryAwsByAccount`
- [x] Add `infracosts.listBillingViews`
- [x] Add `infracosts.getAwsProducts`
- [x] Add `infracosts.getAwsVmPrice`
- [x] Add `infracosts.estimateAwsBoq`
- [x] Sign AWS Price List, Cost Explorer, and Billing API requests with the existing workspace AWS auth layer

### Documentation and role guidance
- [x] Update built-in role descriptions to mention `llmcosts.*`, `infracosts.*`, and `aws_cost_engineer`
- [x] Update README and component diagram for the new cost-context surface
- [ ] Live-verify Artificial Analysis and AWS billing/pricing calls once API credentials are available for both sources
