# Public Release Readiness

Use this checklist before making the repository public or publishing an open-source release.

## Required gates

- [ ] Repository visibility decision is approved.
- [ ] GitHub Actions billing or spending limit is healthy enough for all required checks to start.
- [ ] Required GitHub Actions checks pass on the release PR or release commit.
- [x] CircleCI is the active npm release publisher until npm trusted publishing is fully available.
- [x] Release tags are protected and `rust-v*` tags are created only from reviewed `main` commits.
- [x] The public-history decision is approved and implemented with a sanitized history rewrite.
- [x] Historical secret-scanner findings are resolved for reachable branches and tags.

## Product and data exposure

- [x] Telemetry defaults do not point at live Statsig/OpenTelemetry endpoints.
- [x] Analytics defaults are disabled where public builds should not emit telemetry by default.
- [x] Sentry DSN is supplied by environment instead of being hardcoded.
- [x] Runtime defaults use neutral example endpoints instead of live ThinWedge or ChatGPT backend services.
- [x] Secure devcontainer defaults do not whitelist live production ThinWedge domains.
- [x] Staging hostnames are not embedded in runtime trust logic.
- [x] Real-looking TUI fixture metadata is scrubbed.
- [x] Current-tree scans do not contain live AWS/OpenAI/GitHub/npm/CircleCI credentials.
- [x] Current-tree test fixtures avoid private-key PEM marker blocks that trigger secret scanners.
- [x] Current-tree and history scans block modern OpenAI-shaped keys including `sk-proj-` and `sk-svcacct-` forms.
- [x] CI runs `scripts/check_public_release_readiness.py` against current-tree product/data exposure blockers.
- [x] Current-tree redaction tests avoid provider-shaped fake secret literals.
- [x] Full-history scanner and decision runbook are documented in `docs/public-history-sanitization.md`.
- [x] Full git history no longer contains historical test private-key PEM fixtures on reachable branches and tags.
- [x] Full git history no longer contains fake/test `sk-...` strings on reachable branches and tags.

## Licensing and provenance

- [x] OpenAI Codex attribution is preserved additively in `NOTICE` and `LICENSE`.
- [x] Third-party notices cover OpenAI Codex, Ratatui, WezTerm, ripgrep, and bubblewrap.
- [x] Public-readiness scanning fails if required license/provenance notices are removed.
- [x] npm package staging includes `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md`.
- [x] Public package metadata points at `never2average/fpna-thinwedge` where it is package provenance metadata.
- [x] Public-readiness scanning fails if release-facing package/repository metadata stops pointing at `never2average/fpna-thinwedge`.
- [x] Default public Linux builds do not link vendored bubblewrap into distributed binaries.
- [x] Public-readiness scanning fails if vendored bubblewrap embedding becomes the default again.
- [ ] Any build that opts into vendored bubblewrap with `THINWEDGE_ENABLE_VENDORED_BWRAP=1` has an approved LGPL compliance plan and release artifact set.

## CI and release safety

- [x] CircleCI release publishing is tag-only and validates `rust-v*` tag reachability from `origin/main`.
- [x] CircleCI release publishing builds all CLI npm tarballs before publishing the root wrapper.
- [x] Public-readiness scanning fails if the CircleCI release publisher loses tag reachability, required platform packages, or token hygiene guards.
- [x] Public-readiness scanning fails if external GitHub Actions are not pinned to full commit SHAs.
- [x] GitHub release tag validation requires `rust-v*` tags to be reachable from `origin/main`.
- [x] Public-readiness scanning fails if release tag validation stops requiring `origin/main` reachability.
- [x] npm package publishing checks that all platform tarballs exist before publishing the root wrapper.
- [x] npm platform package target triples are checked against the native release targets hydrated by `install_native_deps.py`.
- [x] npm package staging uses the current GitHub Actions run URL instead of searching for the first matching release workflow run.
- [x] Repository Actions policy requires external actions to be pinned to full commit SHAs.
- [x] Bazel CI no longer enables remote execution by default; private RBE container images are opt-in only.
- [x] Default Bazel PR checks stay on Linux and run one at a time to avoid macOS/Windows hosted-runner spend and local macOS V8 timeouts.
- [x] Release preparation workflows require an explicit `THINWEDGE_BASE_URL` instead of falling back to live ThinWedge or ChatGPT services.
- [x] The GitHub Actions npm publish workflow uses trusted publishing/OIDC instead of long-lived `NPM_TOKEN`.
- [x] Public-readiness scanning fails if GitHub Actions npm workflows regress to `NPM_TOKEN`/`NODE_AUTH_TOKEN` or drop the OIDC publish guard.
- [ ] CircleCI uses a temporary `NPM_TOKEN` project environment variable for npm publishing until trusted publishing is proven.
- [ ] After the repository is public, restrict allowed external actions/reusable workflows to an approved selected-actions list.

## Known external blockers

- GitHub Actions can fail before running any steps if account payments fail or the spending limit is too low. In that case the check annotation points at billing, and code changes cannot make CI green.
- CircleCI project-side statuses require account credits; the release workflow now depends on the public OSS credit grant being active for this project.
- Bazel remote execution remains available only when `THINWEDGE_BAZEL_REMOTE_EXECUTION=1`; the configured RBE container image must be public or otherwise pullable by BuildBuddy.
- Authenticated BuildBuddy cache reads stay enabled, but local-output uploads are disabled by default when remote execution is off to avoid Bazel remote-cache upload crashes in hosted macOS runners.
- The `rust-release-prepare` workflow requires `THINWEDGE_BASE_URL`; configure it deliberately before re-enabling automated model metadata refreshes.
- Public GitHub history was rewritten for existing branches and tags; collaborators must reclone or reset local clones to the rewritten refs.
- npm trusted publishing requires npmjs.com package settings to trust `never2average/fpna-thinwedge` and `.github/workflows/rust-release.yml`; until that is proven, CircleCI publishes with a temporary npm token.
