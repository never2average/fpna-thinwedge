# npm Trusted Publishing Setup

Use this checklist before the next `rust-v*` release publish. The repository workflow is already configured to publish with GitHub Actions OIDC; npm package settings must trust that workflow before the first tokenless publish.

## Trusted publisher identity

Configure `@never2average-does-npm/cli` on npmjs.com with this trusted publisher:

- Provider: GitHub Actions
- Organization or user: `never2average`
- Repository: `fpna-thinwedge`
- Workflow filename: `rust-release.yml`
- Workflow path in this repository: `.github/workflows/rust-release.yml`
- Environment: leave unset unless the release workflow is later moved behind a GitHub Environment

The workflow must continue to run on GitHub-hosted runners. npm trusted publishing does not currently support self-hosted GitHub runners.

## Package and platform versions published by `rust-release.yml`

The current release workflow stages `--package thinwedge`. The root wrapper and platform-specific payloads are all published under the same npm package name, `@never2average-does-npm/cli`, with platform payloads published as platform-suffixed versions and tags. The package uses npm dependency aliases such as `@never2average-does-npm/cli-linux-x64` that point back to `@never2average-does-npm/cli@<version>-linux-x64`; those aliases are not separate npm package settings pages.

For a release like `0.1.11`, the publish set is:

- `@never2average-does-npm/cli@0.1.11`
- `@never2average-does-npm/cli@0.1.11-linux-x64`
- `@never2average-does-npm/cli@0.1.11-linux-arm64`
- `@never2average-does-npm/cli@0.1.11-darwin-x64`
- `@never2average-does-npm/cli@0.1.11-darwin-arm64`
- `@never2average-does-npm/cli@0.1.11-win32-x64`
- `@never2average-does-npm/cli@0.1.11-win32-arm64`

Do not add trusted publishers for packages that the workflow does not currently publish, such as `@thinwedge/thinwedge-sdk` or `@thinwedge/thinwedge-responses-api-proxy`, unless the workflow is updated to stage and publish them.

## Repository-side requirements

These are already handled in `.github/workflows/rust-release.yml`:

- `publish-npm` has `permissions.id-token: write`.
- Node is set to version `24`.
- npm is upgraded to `npm@^11.5.1` before publishing.
- `NODE_AUTH_TOKEN` and `NPM_TOKEN` are not used by the publish step.
- Packages are published with `npm publish --access public`.

## Final cleanup after a successful OIDC publish

After all packages publish successfully through trusted publishing:

1. Remove any obsolete `NPM_TOKEN` GitHub Actions secret for this repository.
2. In npm package settings, consider requiring two-factor authentication and disallowing token publishing.
3. Re-run the release-readiness scan for `NPM_TOKEN` and `NODE_AUTH_TOKEN` references.

Keep `docs/public-release-readiness.md` open until the npm-side trusted publisher setting has been configured and a tokenless publish has been proven.
