# CI and Release Flow

ThinWedge uses GitHub Actions as the public pull-request merge gate and CircleCI
as the release/publish system.

## Pull request gate

The required CI check for pull requests should be the GitHub Actions job
`PR Gate (required)` from `.github/workflows/ci.yml`.

That job is designed to be safe for public contributors and forked PRs. It does
not depend on private release secrets, CircleCI credits, or BuildBuddy-only
credentials.

Useful optional PR signals may still run, including blob-size checks, codespell,
and CLA checks. They are hygiene signals, not the primary CI merge gate.

## Post-merge diagnostics

Heavier workflows run after merge to `main` or by manual dispatch:

- Bazel builds and clippy checks
- full Rust CI matrices
- SDK validation
- cargo-deny advisory/license checks

These workflows are intentionally not the normal public PR gate. If they fail on
`main`, track that as a follow-up and avoid blocking unrelated contributor PRs
with known base-branch failures.

## Release and publish

CircleCI owns the automatic `rust-v*` release path. A valid release tag must be
reachable from `origin/main` and must match the version in `thinwedge-rs/Cargo.toml`.

CircleCI then builds the platform binaries, stages release assets, publishes the
GitHub release, and publishes the npm packages.

Use `rust-v*` only when the native CLI binaries need to change. It rebuilds the
full Linux, macOS, and Windows matrix.

For README, package metadata, npm-page, or install-copy changes that do not touch
native binaries, use an `npm-v*` tag instead. The `npm-v*` path publishes only the
root npm package and reuses the latest already-published platform packages through
`optionalDependencies`, so it avoids the expensive native Rust matrix.

The old GitHub Actions release workflow has been removed. Do not add an
automatic GitHub tag-release workflow unless CircleCI publishing is retired in
the same PR.

## Monitoring handoff

For new pull requests, monitor GitHub Actions first and look for
`PR Gate (required)`. For release tags, monitor CircleCI.
