#!/usr/bin/env python3
"""Write the ThinWedge Homebrew formula for a release."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


ASSET_NAMES = {
    "macos_arm64": "thinwedge-aarch64-apple-darwin.tar.gz",
    "macos_x64": "thinwedge-x86_64-apple-darwin.tar.gz",
    "linux_arm64": "thinwedge-aarch64-unknown-linux-musl.tar.gz",
    "linux_x64": "thinwedge-x86_64-unknown-linux-gnu.tar.gz",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--repo", required=True, help="GitHub repo slug, e.g. never2average/fpna-thinwedge")
    parser.add_argument("--assets-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    assets_dir = args.assets_dir.resolve()
    checksums = {key: sha256(assets_dir / name) for key, name in ASSET_NAMES.items()}
    base_url = f"https://github.com/{args.repo}/releases/download/rust-v{args.version}"

    content = f"""class Thinwedge < Formula
  desc "ThinWedge FP&A agent terminal"
  homepage "https://github.com/{args.repo}"
  version "{args.version}"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "{base_url}/{ASSET_NAMES['macos_arm64']}"
      sha256 "{checksums['macos_arm64']}"
    else
      url "{base_url}/{ASSET_NAMES['macos_x64']}"
      sha256 "{checksums['macos_x64']}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "{base_url}/{ASSET_NAMES['linux_arm64']}"
      sha256 "{checksums['linux_arm64']}"
    else
      url "{base_url}/{ASSET_NAMES['linux_x64']}"
      sha256 "{checksums['linux_x64']}"
    end
  end

  def install
    binary_name =
      if OS.mac?
        Hardware::CPU.arm? ? "thinwedge-aarch64-apple-darwin" : "thinwedge-x86_64-apple-darwin"
      else
        Hardware::CPU.arm? ? "thinwedge-aarch64-unknown-linux-musl" : "thinwedge-x86_64-unknown-linux-gnu"
      end

    bin.install binary_name => "thinwedge"
  end

  test do
    assert_match version.to_s, shell_output("#{{bin}}/thinwedge --version")
  end
end
"""

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
