#!/bin/bash

# Set "chatgpt.cliExecutable": "/Users/<USERNAME>/code/thinwedge/scripts/debug-thinwedge.sh" in VSCode settings to always get the 
# latest thinwedge-rs binary when debugging ThinWedge Extension.


set -euo pipefail

THINWEDGE_RS_DIR=$(realpath "$(dirname "$0")/../thinwedge-rs")
(cd "$THINWEDGE_RS_DIR" && cargo run --quiet --bin thinwedge -- "$@")
