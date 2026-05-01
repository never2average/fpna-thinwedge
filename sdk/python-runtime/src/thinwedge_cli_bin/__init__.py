from __future__ import annotations

import os
from pathlib import Path

PACKAGE_NAME = "openai-thinwedge-cli-bin"


def bundled_thinwedge_path() -> Path:
    exe = "thinwedge.exe" if os.name == "nt" else "thinwedge"
    path = Path(__file__).resolve().parent / "bin" / exe
    if not path.is_file():
        raise FileNotFoundError(
            f"{PACKAGE_NAME} is installed but missing its packaged thinwedge binary at {path}"
        )
    return path


__all__ = ["PACKAGE_NAME", "bundled_thinwedge_path"]
