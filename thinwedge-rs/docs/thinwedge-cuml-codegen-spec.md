# ThinWedge cuML Codegen Spec

Last reviewed: April 30, 2026

This document defines how ThinWedge should generate code for RAPIDS / cuML workloads that run inside Runpod Pods.

## Scope

Use this spec when the target model is:

- tabular
- sklearn-like
- dataframe-heavy
- feature-engineering-heavy
- naturally expressed with pandas-style code

Default runtime family:

- `thinwedge-rapids-cu12`

## Baseline Strategy

ThinWedge should generate to the safest baseline first:

1. `cudf.pandas` for dataframe execution
2. `cuml.accel` for sklearn-style estimators
3. direct `cuML` only when the estimator and workflow are intentionally GPU-native

This gives the code a cleaner fallback story and reduces the chance that generated code depends on unsupported low-level RAPIDS behavior.

## Required Bootstrap Pattern

Generated scripts must enable acceleration before importing `pandas` or `sklearn`.

Required top-of-file pattern:

```python
import cudf.pandas
cudf.pandas.install()

import cuml
cuml.accel.install(log_level="info")
```

Only after that may the script import:

- `pandas`
- `sklearn`
- `umap`
- `hdbscan`

## Direct `cuML` Mode

If ThinWedge intentionally generates a direct GPU-native estimator, import it explicitly:

```python
from cuml.ensemble import RandomForestRegressor
from cuml.metrics import mean_squared_error
```

Use direct `cuML` when:

- the estimator is well-supported in `cuML`
- GPU execution is required
- determinism/performance matters more than sklearn portability

## Preferred Code Shape

Generated code should:

- use vectorized dataframe operations
- keep feature engineering in dataframe expressions
- keep training and prediction in explicit functions
- separate dataset loading, feature preparation, training, evaluation, and serialization

Recommended module shape:

```python
def load_dataset(input_path: str) -> "pd.DataFrame": ...
def build_features(df): ...
def train_model(features, labels): ...
def evaluate_model(model, features, labels): ...
def save_artifacts(output_root: str, model, metrics: dict): ...
```

## Forbidden Patterns

Do not generate:

- row-wise Python loops over dataframe rows
- `iterrows()` or `itertuples()` for core data transforms
- mixed-mode code that jumps between `pandas` and `cudf` arbitrarily
- ad hoc package installation inside the training or batch job
- writes outside `/workspace`
- hidden fallback-only semantics

Avoid:

- mutating pandas UDF assumptions that rely on CPU-only behavior
- order-sensitive joins without an explicit post-join sort

## Data And Artifact Paths

Generated code should use these locations:

- input datasets: `/workspace/thinwedge/datasets/...`
- checkpoints or persisted models: `/workspace/thinwedge/checkpoints/...`
- job outputs: `/workspace/thinwedge/outputs/<jobId>/...`
- eval summaries: `/workspace/thinwedge/evals/<jobId>.json`

The code should accept these as parameters instead of hardcoding job-specific paths internally.

## Serialization Rules

Preferred outputs:

- metrics JSON
- feature manifest JSON
- model artifact path manifest JSON

When persisting models:

- use a deliberate serialization path
- document whether the artifact is RAPIDS-only or CPU-portable
- do not assume arbitrary pickle loading from untrusted sources

## Validation And Profiling

ThinWedge-generated validation steps should include:

- dataset row/column counts
- null/NaN summary
- feature count summary
- eval metrics

Useful runtime profiling commands:

```bash
python -m cudf.pandas --profile /workspace/thinwedge/model-repo/train.py
python -m cuml.accel -v --profile /workspace/thinwedge/model-repo/train.py
```

If strict GPU-only behavior is required, allow:

```bash
export CUDF_PANDAS_FAIL_ON_FALLBACK=1
```

## Batch Inference Expectations

For batch inference, generated cuML code should:

- load one input shard at a time
- process bounded dataframe chunks
- write incremental outputs
- avoid keeping the full workload in memory when not necessary

Preferred batch entrypoint shape:

```python
def run_batch_inference(
    input_path: str,
    output_path: str,
    model_path: str,
    batch_size: int | None = None,
    shard_index: int | None = None,
    shard_count: int | None = None,
) -> dict:
    ...
```

## ThinWedge Prompting Guidance

When ThinWedge asks an LLM to generate a cuML model, the prompt should require:

- RAPIDS-compatible code only
- top-of-file accelerator bootstrap
- no runtime dependency installation
- no filesystem writes outside `/workspace`
- deterministic output paths supplied by ThinWedge
- explicit training/eval/save functions

## Sources

- RAPIDS install guide: https://docs.rapids.ai/install/
- RAPIDS platform support: https://docs.rapids.ai/platform-support/
- `cudf.pandas` docs: https://docs.rapids.ai/api/cudf/stable/cudf_pandas/
- `cudf.pandas` usage: https://docs.rapids.ai/api/cudf/stable/cudf_pandas/usage/
- cuML docs: https://docs.rapids.ai/api/cuml/stable/
- `cuml.accel` docs: https://docs.rapids.ai/api/cuml/stable/cuml-accel/
- cuML supported versions: https://docs.rapids.ai/api/cuml/stable/supported_versions/
