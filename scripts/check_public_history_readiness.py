#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass


DEFAULT_MAX_BLOB_BYTES = 2 * 1024 * 1024
DEFAULT_SAMPLE_LIMIT = 10


@dataclass(frozen=True)
class HistoryPattern:
    name: str
    pattern: re.Pattern[bytes]
    description: str


@dataclass(frozen=True)
class BlobInfo:
    object_id: str
    path: str
    size: int


PATTERNS = [
    HistoryPattern(
        "private-key-pem",
        re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
        "Historical blob contains a private-key PEM marker.",
    ),
    HistoryPattern(
        "circleci-pat",
        re.compile(rb"C" + rb"CIPAT_[A-Za-z0-9_]+"),
        "Historical blob contains a CircleCI personal access token shape.",
    ),
    HistoryPattern(
        "github-token",
        re.compile(rb"(?:github_pat_|gh[pousr]_)[A-Za-z0-9_]{20,}"),
        "Historical blob contains a GitHub access token shape.",
    ),
    HistoryPattern(
        "npm-token",
        re.compile(rb"npm_[A-Za-z0-9]{30,}"),
        "Historical blob contains an npm access token shape.",
    ),
    HistoryPattern(
        "aws-access-key",
        re.compile(rb"A[SK]IA[0-9A-Z]{16}"),
        "Historical blob contains an AWS access key shape.",
    ),
    HistoryPattern(
        "openai-token-shape",
        re.compile(rb"sk-[A-Za-z0-9]{20,}"),
        "Historical blob contains an OpenAI-shaped token string.",
    ),
]


def run_git(*args: str, input_bytes: bytes | None = None) -> bytes:
    return subprocess.run(
        ["git", *args],
        input=input_bytes,
        check=True,
        capture_output=True,
    ).stdout


def collect_object_paths() -> dict[str, str]:
    paths: dict[str, str] = {}
    for raw_line in run_git("rev-list", "--objects", "--all").splitlines():
        parts = raw_line.split(b" ", 1)
        object_id = parts[0].decode("ascii")
        if len(parts) == 1:
            paths.setdefault(object_id, "")
            continue
        path = parts[1].decode("utf-8", "replace")
        paths.setdefault(object_id, path)
    return paths


def collect_blobs(max_blob_bytes: int) -> list[BlobInfo]:
    paths = collect_object_paths()
    object_ids = sorted(paths)
    batch_input = ("\n".join(object_ids) + "\n").encode("ascii")
    checked = run_git("cat-file", "--batch-check", input_bytes=batch_input)

    blobs: list[BlobInfo] = []
    for raw_line in checked.splitlines():
        object_id, object_type, size_text, *_ = raw_line.split(maxsplit=3)
        if object_type != b"blob":
            continue
        size = int(size_text)
        if size > max_blob_bytes:
            continue
        decoded_id = object_id.decode("ascii")
        blobs.append(BlobInfo(decoded_id, paths.get(decoded_id, ""), size))
    return blobs


def scan_blobs(blobs: list[BlobInfo], sample_limit: int) -> dict[str, list[str]]:
    samples = {pattern.name: [] for pattern in PATTERNS}
    counts = {pattern.name: 0 for pattern in PATTERNS}

    process = subprocess.Popen(
        ["git", "cat-file", "--batch"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert process.stdin is not None
    assert process.stdout is not None

    for blob in blobs:
        process.stdin.write(f"{blob.object_id}\n".encode("ascii"))
        process.stdin.flush()

        header = process.stdout.readline()
        if not header:
            raise RuntimeError("git cat-file ended before all blobs were scanned")
        object_id, object_type, size_text = header.split()[:3]
        size = int(size_text)
        contents = process.stdout.read(size)
        process.stdout.read(1)

        if object_type != b"blob":
            continue

        for pattern in PATTERNS:
            count = len(pattern.pattern.findall(contents))
            if not count:
                continue
            counts[pattern.name] += count
            if len(samples[pattern.name]) < sample_limit:
                path = blob.path or "<unknown path>"
                samples[pattern.name].append(
                    f"{object_id.decode('ascii')[:12]} {path} count={count}"
                )

    process.stdin.close()
    process.wait()

    return {
        pattern.name: [f"total={counts[pattern.name]}", *samples[pattern.name]]
        for pattern in PATTERNS
        if counts[pattern.name]
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Scan reachable git history for public-release secret blockers."
    )
    parser.add_argument(
        "--max-blob-bytes",
        type=int,
        default=DEFAULT_MAX_BLOB_BYTES,
        help=f"Skip blobs larger than this value. Default: {DEFAULT_MAX_BLOB_BYTES}.",
    )
    parser.add_argument(
        "--sample-limit",
        type=int,
        default=DEFAULT_SAMPLE_LIMIT,
        help=f"Maximum samples to print per finding type. Default: {DEFAULT_SAMPLE_LIMIT}.",
    )
    args = parser.parse_args()

    blobs = collect_blobs(args.max_blob_bytes)
    findings = scan_blobs(blobs, args.sample_limit)

    if not findings:
        print(
            "Public history readiness scan passed "
            f"({len(blobs)} historical blobs checked)."
        )
        return 0

    print(
        "Public history readiness scan failed "
        f"({len(blobs)} historical blobs checked):"
    )
    descriptions = {pattern.name: pattern.description for pattern in PATTERNS}
    for name, entries in findings.items():
        print(f"- {name}: {descriptions[name]}")
        for entry in entries:
            print(f"  {entry}")
    print(
        "\nResolve these findings with a history rewrite, publish from a sanitized "
        "history branch, or record explicit owner acceptance before making the "
        "repository public."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
