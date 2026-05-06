#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


MAX_TEXT_BYTES = 2 * 1024 * 1024
PINNED_ACTION_REF = re.compile(r"^[0-9a-f]{40}$")
USES_LINE = re.compile(r"^\s*uses:\s*['\"]?([^'\"\s#]+)", re.MULTILINE)


@dataclass(frozen=True)
class BlockedPattern:
    name: str
    pattern: re.Pattern[bytes]
    description: str


def literal(value: str) -> re.Pattern[bytes]:
    return re.compile(re.escape(value.encode("utf-8")))


BLOCKED_PATTERNS = [
    BlockedPattern(
        "staging-host-trust",
        literal("chatgpt-staging" + ".com"),
        "Staging hostnames must not be embedded in runtime trust logic.",
    ),
    BlockedPattern(
        "operator-local-path-fixture",
        literal("/Users/" + "easong"),
        "Public fixtures must not expose real-looking operator local paths.",
    ),
    BlockedPattern(
        "operator-session-fixture",
        literal("8f7c4ac2-6141-42da-" + "b4d5-7032a8e8df3b"),
        "Public fixtures must not expose real-looking session IDs.",
    ),
    BlockedPattern(
        "operator-history-fixture",
        literal("253" + "2619"),
        "Public fixtures must not expose real-looking history IDs.",
    ),
    BlockedPattern(
        "statsig-otlp-endpoint",
        literal("ab.chatgpt" + ".com/otlp"),
        "Telemetry defaults must not point at live Statsig/OpenTelemetry endpoints.",
    ),
    BlockedPattern(
        "statsig-client-key",
        literal("client-" + "MkRule"),
        "Telemetry defaults must not include a hardcoded Statsig client key.",
    ),
    BlockedPattern(
        "statsig-sdk-key",
        literal("ae32ed50620d7a7792c1ce5" + "df38b3e3e"),
        "Telemetry defaults must not include a hardcoded Statsig SDK key.",
    ),
    BlockedPattern(
        "sentry-ingest-host",
        literal("ingest.us.sentry" + ".io"),
        "Sentry must be supplied by environment instead of a hardcoded DSN.",
    ),
    BlockedPattern(
        "private-key-pem-fixture",
        re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
        "Current-tree test fixtures must avoid private-key PEM marker blocks.",
    ),
    BlockedPattern(
        "curated-plugin-live-backend-fallback",
        literal("chatgpt.com/backend-api/plugins/export/" + "curated"),
        "Runtime defaults must not fall back to live ChatGPT backend services.",
    ),
    BlockedPattern(
        "release-prepare-live-base-url-default",
        literal("THINWEDGE_BASE_URL:-https://chatgpt.com/backend-api/" + "thinwedge"),
        "Release preparation workflows must not default to live ChatGPT backend services.",
    ),
    BlockedPattern(
        "devcontainer-live-domain-default",
        literal(
            'THINWEDGE_ALLOWED_DOMAINS="${THINWEDGE_ALLOWED_DOMAINS:-'
            + "api.thinwedge.com}"
        ),
        "Secure devcontainer defaults must not whitelist live production domains.",
    ),
    BlockedPattern(
        "firewall-live-domain-default",
        literal('ALLOWED_DOMAINS=("' + "api.thinwedge.com" + '")'),
        "Firewall defaults must not silently allow live production domains.",
    ),
    BlockedPattern(
        "firewall-live-domain-smoke-test",
        literal("curl --connect-timeout 5 https://" + "api.thinwedge.com"),
        "Firewall verification must use the configured domain, not a live production domain.",
    ),
    BlockedPattern(
        "bazel-ci-upstream-repo-url",
        literal("REPO_URL=https://github.com/" + "thinwedge/thinwedge.git"),
        "Bazel CI provenance metadata must point at this repository.",
    ),
    BlockedPattern(
        "python-runtime-upstream-release-slug",
        literal('REPO_SLUG = "' + "thinwedge/thinwedge" + '"'),
        "Python runtime setup must resolve release artifacts from this repository.",
    ),
    BlockedPattern(
        "install-doc-upstream-clone-url",
        literal("git clone https://github.com/" + "thinwedge/thinwedge.git"),
        "Install docs must clone this repository.",
    ),
    BlockedPattern(
        "changelog-upstream-release-url",
        literal(
            "[releases page](https://github.com/"
            + "thinwedge/thinwedge/releases)"
        ),
        "Release-facing docs must point at this repository's releases.",
    ),
    BlockedPattern(
        "circleci-pat",
        re.compile(rb"C" + rb"CIPAT_[A-Za-z0-9_]+"),
        "CircleCI personal access tokens must not be committed.",
    ),
    BlockedPattern(
        "github-token",
        re.compile(rb"(?:github_pat_|gh[pousr]_)[A-Za-z0-9_]{20,}"),
        "GitHub access tokens must not be committed.",
    ),
    BlockedPattern(
        "npm-token",
        re.compile(rb"npm_[A-Za-z0-9]{30,}"),
        "npm access tokens must not be committed.",
    ),
    BlockedPattern(
        "aws-access-key",
        re.compile(rb"A[SK]IA[0-9A-Z]{16}"),
        "AWS access keys must not be committed.",
    ),
    BlockedPattern(
        "openai-token-shape",
        re.compile(rb"sk-[A-Za-z0-9]{20,}"),
        "OpenAI-shaped tokens must not be committed, even as test fixtures.",
    ),
]


def run_git(*args: str) -> bytes:
    return subprocess.run(
        ["git", *args],
        check=True,
        capture_output=True,
    ).stdout


def tracked_files() -> list[Path]:
    output = run_git("ls-files", "-z")
    return [Path(raw.decode("utf-8")) for raw in output.split(b"\0") if raw]


def line_number(contents: bytes, offset: int) -> int:
    return contents.count(b"\n", 0, offset) + 1


def is_scannable(path: Path) -> bool:
    if path.parts and path.parts[0] in {"target", "node_modules"}:
        return False
    return True


def load_python_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def check_npm_platform_targets() -> list[str]:
    build_module = load_python_module(
        "thinwedge_build_npm_package",
        Path("thinwedge-cli/scripts/build_npm_package.py"),
    )
    install_module = load_python_module(
        "thinwedge_install_native_deps",
        Path("thinwedge-cli/scripts/install_native_deps.py"),
    )

    platform_packages = getattr(build_module, "THINWEDGE_PLATFORM_PACKAGES", {})
    binary_targets = set(getattr(install_module, "BINARY_TARGETS", ()))

    findings: list[str] = []
    for package, config in sorted(platform_packages.items()):
        target = config.get("target_triple")
        if target not in binary_targets:
            findings.append(
                f"{package} targets {target}, but install_native_deps.py only "
                "hydrates release binaries for: "
                + ", ".join(sorted(binary_targets))
            )

    return findings


def check_release_workflow_npm_staging() -> list[str]:
    workflow = Path(".github/workflows/rust-release.yml").read_text(encoding="utf-8")
    expected = (
        '--workflow-url "${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/'
        '${GITHUB_RUN_ID}"'
    )
    if expected in workflow:
        return []
    return [
        "rust-release.yml must pass the current GitHub Actions run URL to "
        "stage_npm_packages.py so release artifacts cannot be resolved from a "
        "stale or unrelated workflow run."
    ]


def check_circleci_filtered_out() -> list[str]:
    config_path = Path(".circleci/config.yml")
    if not config_path.exists():
        return []

    config = config_path.read_text(encoding="utf-8")
    required_snippets = (
        "branches:\n              ignore: /.*/",
        "tags:\n              ignore: /.*/",
    )
    if all(snippet in config for snippet in required_snippets):
        return []
    return [
        ".circleci/config.yml must keep the remaining smoke job filtered out "
        "for all branches and tags so public readiness does not depend on "
        "CircleCI credits or project-side status settings."
    ]


def check_github_actions_pinned() -> list[str]:
    findings: list[str] = []
    github_dirs = (Path(".github/workflows"), Path(".github/actions"))

    for github_dir in github_dirs:
        if not github_dir.exists():
            continue

        for path in sorted(github_dir.rglob("*")):
            if path.suffix not in {".yml", ".yaml"}:
                continue

            contents = path.read_text(encoding="utf-8")
            for match in USES_LINE.finditer(contents):
                uses_value = match.group(1)
                if uses_value.startswith(("./", "docker://")):
                    continue

                if "@" not in uses_value:
                    findings.append(f"{path}: external action '{uses_value}' is missing a ref.")
                    continue

                action_name, action_ref = uses_value.rsplit("@", 1)
                if not PINNED_ACTION_REF.fullmatch(action_ref):
                    findings.append(
                        f"{path}: external action '{action_name}' must be pinned to a "
                        f"40-character commit SHA instead of '{action_ref}'."
                    )

    return findings


def main() -> int:
    findings: list[tuple[Path, int, BlockedPattern]] = []
    structural_findings = [
        *check_npm_platform_targets(),
        *check_release_workflow_npm_staging(),
        *check_circleci_filtered_out(),
        *check_github_actions_pinned(),
    ]
    checked = 0

    for path in tracked_files():
        if not is_scannable(path):
            continue
        try:
            contents = path.read_bytes()
        except OSError as exc:
            print(f"warning: could not read {path}: {exc}", file=sys.stderr)
            continue
        if len(contents) > MAX_TEXT_BYTES or b"\0" in contents:
            continue

        checked += 1
        for blocked in BLOCKED_PATTERNS:
            match = blocked.pattern.search(contents)
            if match:
                findings.append((path, line_number(contents, match.start()), blocked))

    if not findings and not structural_findings:
        print(f"Public release readiness scan passed ({checked} tracked text files checked).")
        return 0

    print("Public release readiness scan failed:")
    for finding in structural_findings:
        print(f"- structural-release-readiness: {finding}")
    for path, line, blocked in findings:
        print(f"- {path}:{line}: {blocked.name}: {blocked.description}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
