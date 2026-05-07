# npm Trusted Publishing Notes

npm trusted publishing is not active for this repository right now. The old
GitHub Actions release workflow has been removed, and CircleCI is the current
release and npm publish path for `rust-v*` tags.

## Current release publisher

CircleCI publishes the CLI packages with a project-scoped `NPM_TOKEN` environment
variable. Keep that token scoped to npm publish permissions for the
`@never2average-does-npm/cli` package and rotate it after any suspected exposure.

For a release like `0.1.11`, CircleCI publishes:

- `@never2average-does-npm/cli@0.1.11`
- `@never2average-does-npm/cli@0.1.11-linux-x64`
- `@never2average-does-npm/cli@0.1.11-linux-arm64`
- `@never2average-does-npm/cli@0.1.11-darwin-x64`
- `@never2average-does-npm/cli@0.1.11-darwin-arm64`
- `@never2average-does-npm/cli@0.1.11-win32-x64`
- `@never2average-does-npm/cli@0.1.11-win32-arm64`

## Future trusted-publishing migration

If this repo later moves npm publishing back to GitHub Actions, create a new
workflow and configure npmjs.com to trust that exact workflow filename. Do not
reuse references to the removed `.github/workflows/rust-release.yml` workflow.

A future migration PR should also update:

- `.github/workflows/README.md`
- `docs/ci-release-flow.md`
- `docs/public-release-readiness.md`
- `scripts/check_public_release_readiness.py`

Until that migration is implemented and proven by a tokenless publish, CircleCI
remains the source of truth for release publishing.
