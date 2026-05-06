# Public Release Readiness

Use this checklist before making the repository public or publishing an open-source release.

## Required gates

- [ ] Repository visibility decision is approved.
- [ ] GitHub Actions billing or spending limit is healthy enough for all required checks to start.
- [ ] Required GitHub Actions checks pass on the release PR or release commit.
- [x] CircleCI is not a release authority.
- [ ] Release tags are protected and `rust-v*` tags are created only from reviewed `main` commits; protection setup is blocked while the repo is private on the current GitHub plan.
- [ ] The public-history decision is approved: either expose inherited history intentionally, or publish from a sanitized history branch.
- [ ] Historical secret-scanner findings are resolved or explicitly accepted before public visibility is enabled.

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
- [ ] Full git history no longer contains historical test private-key PEM fixtures, or the release owner has explicitly accepted the inherited-history exposure.
- [x] Full git history token-pattern review found fake/test `sk-...` strings but no evidence of live provider tokens.

## Licensing and provenance

- [x] OpenAI Codex attribution is preserved additively in `NOTICE` and `LICENSE`.
- [x] Third-party notices cover OpenAI Codex, Ratatui, WezTerm, ripgrep, and bubblewrap.
- [x] npm package staging includes `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md`.
- [x] Public package metadata points at `never2average/fpna-thinwedge` where it is package provenance metadata.
- [x] Default public Linux builds do not link vendored bubblewrap into distributed binaries.
- [ ] Any build that opts into vendored bubblewrap with `THINWEDGE_ENABLE_VENDORED_BWRAP=1` has an approved LGPL compliance plan and release artifact set.

## CI and release safety

- [x] CircleCI release publishing is retired; CircleCI is smoke-only.
- [x] GitHub release tag validation requires `rust-v*` tags to be reachable from `origin/main`.
- [x] npm package publishing checks that all platform tarballs exist before publishing the root wrapper.
- [x] Repository Actions policy requires external actions to be pinned to full commit SHAs.
- [ ] Prefer npm trusted publishing/OIDC over long-lived `NPM_TOKEN`, or explicitly accept the token risk.
- [ ] After the repository is public, restrict allowed external actions/reusable workflows to an approved selected-actions list.

## Known external blockers

- GitHub Actions can fail before running any steps if account payments fail or the spending limit is too low. In that case the check annotation points at billing, and code changes cannot make CI green.
- CircleCI smoke logs require a valid CircleCI token or project access outside this repository checkout.
- Public GitHub history cannot hide old committed test private-key PEM fixtures without a history rewrite or sanitized publication branch.
- GitHub branch protection and repository rulesets return `403` while this private repository is on a plan that requires GitHub Pro or public visibility for those features.
