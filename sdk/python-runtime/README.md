# ThinWedge CLI Runtime for Python SDK

Platform-specific runtime package consumed by the published `thinwedge-thinwedge-app-server-sdk`.

This package is staged during release so the SDK can pin an exact ThinWedge CLI
version without checking platform binaries into the repo.

`thinwedge-thinwedge-cli-bin` is intentionally wheel-only. Do not build or publish an
sdist for this package.
