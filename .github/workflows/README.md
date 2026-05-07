# Workflow Strategy

This repository uses one primary pre-merge CI path and one release/publish path.

## Pull Requests

- `ci.yml` is the primary GitHub Actions merge gate. The required job should be
  `PR Gate (required)`.
- The PR gate is intentionally lightweight and stable for public contributors:
  repository policy checks, manifest boundary checks, package staging, README
  checks, packaging checks, and formatting checks that do not require private
  CI credentials.
- `blob-size-policy.yml`, `codespell.yml`, and `cla.yml` can still provide
  contributor hygiene signal, but the required CI merge gate should remain the
  single `PR Gate (required)` job unless maintainers deliberately change branch
  protection.

## Post-Merge On `main`

- Heavy GitHub Actions workflows run on `main` and by manual dispatch, not on
  every contributor PR. This keeps public PRs reviewable while still preserving
  deeper coverage after merge.
- `bazel.yml`, `rust-ci.yml`, `rust-ci-full.yml`, `sdk.yml`, and
  `cargo-deny.yml` are post-merge/manual diagnostics. If one of these fails on
  `main`, open or track a follow-up instead of blocking unrelated public PRs.

## Releases

- CircleCI is the primary release and publish system. Pushing a valid `rust-v*`
  tag starts the CircleCI release workflow, builds platform binaries, publishes
  GitHub release assets, and publishes npm packages.
- The old GitHub Actions release workflow has been removed. Do not add an
  automatic GitHub tag-release workflow unless CircleCI publishing is retired in
  the same PR.

## Monitoring

After this flow is merged, new public PRs should be monitored by checking the
`PR Gate (required)` job first. CircleCI should only need attention for release
tags or explicit release reruns.
