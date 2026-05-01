# Containerized Development

We provide two container paths:

- `devcontainer.json` keeps the existing ThinWedge contributor setup for working on this repository.
- `devcontainer.secure.json` adds a customer-oriented profile with stricter outbound network controls.

## ThinWedge contributor profile

Use `devcontainer.json` when you are developing ThinWedge itself. This is the same lightweight arm64 container that already exists in the repo.

## Secure customer profile

Use `devcontainer.secure.json` when you want a stricter runtime profile for running ThinWedge inside a project container:

- installs the ThinWedge CLI plus common build tools
- installs bubblewrap in setuid mode for ThinWedge's Linux sandbox
- disables Docker's outer seccomp and AppArmor profiles so bubblewrap can construct ThinWedge's inner sandbox
- enables firewall startup with an allowlist-driven outbound policy
- blocks IPv6 by default so the allowlist cannot be bypassed over AAAA routes
- requires `NET_ADMIN` and `NET_RAW` so the firewall can be installed at startup

This profile keeps the stricter networking isolated to the customer path instead of changing the default ThinWedge contributor container.

Start it from the CLI with:

```bash
devcontainer up --workspace-folder . --config .devcontainer/devcontainer.secure.json
```

In VS Code, choose **Dev Containers: Open Folder in Container...** and select `.devcontainer/devcontainer.secure.json`.

## Docker

To build the contributor image locally for x64 and then run it with the repo mounted under `/workspace`:

```shell
THINWEDGE_DOCKER_IMAGE_NAME=thinwedge-linux-dev
docker build --platform=linux/amd64 -t "$THINWEDGE_DOCKER_IMAGE_NAME" ./.devcontainer
docker run --platform=linux/amd64 --rm -it -e CARGO_TARGET_DIR=/workspace/thinwedge-rs/target-amd64 -v "$PWD":/workspace -w /workspace/thinwedge-rs "$THINWEDGE_DOCKER_IMAGE_NAME"
```

Note that `/workspace/target` will contain the binaries built for your host platform, so we include `-e CARGO_TARGET_DIR=/workspace/thinwedge-rs/target-amd64` in the `docker run` command so that the binaries built inside your container are written to a separate directory.

For arm64, specify `--platform=linux/arm64` instead for both `docker build` and `docker run`.

Currently, the contributor `Dockerfile` works for both x64 and arm64 Linux, though you need to run `rustup target add x86_64-unknown-linux-musl` yourself to install the musl toolchain for x64.

The secure profile's capability, seccomp, and AppArmor options are required when you want ThinWedge's bubblewrap sandbox to run inside Docker as the non-root devcontainer user. Without them, Docker's default runtime profile can block bubblewrap's namespace setup before ThinWedge's own seccomp filter is installed. This keeps the Docker relaxation explicit in the profile that is meant to run ThinWedge inside a project container, while the default contributor profile stays lightweight.
