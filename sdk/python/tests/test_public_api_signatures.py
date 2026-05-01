from __future__ import annotations

import importlib.resources as resources
import inspect
import tomllib
from pathlib import Path
from typing import Any

import thinwedge_app_server
from thinwedge_app_server import AppServerConfig, RunResult
from thinwedge_app_server.models import InitializeResponse
from thinwedge_app_server.api import AsyncThinWedge, AsyncThread, ThinWedge, Thread


def _keyword_only_names(fn: object) -> list[str]:
    signature = inspect.signature(fn)
    return [
        param.name
        for param in signature.parameters.values()
        if param.kind == inspect.Parameter.KEYWORD_ONLY
    ]


def _assert_no_any_annotations(fn: object) -> None:
    signature = inspect.signature(fn)
    for param in signature.parameters.values():
        if param.annotation is Any:
            raise AssertionError(
                f"{fn} has public parameter typed as Any: {param.name}"
            )
    if signature.return_annotation is Any:
        raise AssertionError(f"{fn} has public return annotation typed as Any")


def test_root_exports_app_server_config() -> None:
    assert AppServerConfig.__name__ == "AppServerConfig"


def test_root_exports_run_result() -> None:
    assert RunResult.__name__ == "RunResult"


def test_package_and_default_client_versions_follow_project_version() -> None:
    pyproject_path = Path(__file__).resolve().parents[1] / "pyproject.toml"
    pyproject = tomllib.loads(pyproject_path.read_text())

    assert thinwedge_app_server.__version__ == pyproject["project"]["version"]
    assert AppServerConfig().client_version == thinwedge_app_server.__version__


def test_package_includes_py_typed_marker() -> None:
    marker = resources.files("thinwedge_app_server").joinpath("py.typed")
    assert marker.is_file()


def test_generated_public_signatures_are_snake_case_and_typed() -> None:
    expected = {
        ThinWedge.thread_start: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "ephemeral",
            "model",
            "model_provider",
            "permission_profile",
            "personality",
            "sandbox",
            "service_name",
            "service_tier",
            "session_start_source",
        ],
        ThinWedge.thread_list: [
            "archived",
            "cursor",
            "cwd",
            "limit",
            "model_providers",
            "search_term",
            "sort_direction",
            "sort_key",
            "source_kinds",
            "use_state_db_only",
        ],
        ThinWedge.thread_resume: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "exclude_turns",
            "model",
            "model_provider",
            "permission_profile",
            "personality",
            "sandbox",
            "service_tier",
        ],
        ThinWedge.thread_fork: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "ephemeral",
            "exclude_turns",
            "model",
            "model_provider",
            "permission_profile",
            "sandbox",
            "service_tier",
        ],
        Thread.turn: [
            "approval_policy",
            "approvals_reviewer",
            "cwd",
            "effort",
            "model",
            "output_schema",
            "permission_profile",
            "personality",
            "sandbox_policy",
            "service_tier",
            "summary",
        ],
        Thread.run: [
            "approval_policy",
            "approvals_reviewer",
            "cwd",
            "effort",
            "model",
            "output_schema",
            "permission_profile",
            "personality",
            "sandbox_policy",
            "service_tier",
            "summary",
        ],
        AsyncThinWedge.thread_start: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "ephemeral",
            "model",
            "model_provider",
            "permission_profile",
            "personality",
            "sandbox",
            "service_name",
            "service_tier",
            "session_start_source",
        ],
        AsyncThinWedge.thread_list: [
            "archived",
            "cursor",
            "cwd",
            "limit",
            "model_providers",
            "search_term",
            "sort_direction",
            "sort_key",
            "source_kinds",
            "use_state_db_only",
        ],
        AsyncThinWedge.thread_resume: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "exclude_turns",
            "model",
            "model_provider",
            "permission_profile",
            "personality",
            "sandbox",
            "service_tier",
        ],
        AsyncThinWedge.thread_fork: [
            "approval_policy",
            "approvals_reviewer",
            "base_instructions",
            "config",
            "cwd",
            "developer_instructions",
            "ephemeral",
            "exclude_turns",
            "model",
            "model_provider",
            "permission_profile",
            "sandbox",
            "service_tier",
        ],
        AsyncThread.turn: [
            "approval_policy",
            "approvals_reviewer",
            "cwd",
            "effort",
            "model",
            "output_schema",
            "permission_profile",
            "personality",
            "sandbox_policy",
            "service_tier",
            "summary",
        ],
        AsyncThread.run: [
            "approval_policy",
            "approvals_reviewer",
            "cwd",
            "effort",
            "model",
            "output_schema",
            "permission_profile",
            "personality",
            "sandbox_policy",
            "service_tier",
            "summary",
        ],
    }

    for fn, expected_kwargs in expected.items():
        actual = _keyword_only_names(fn)
        assert actual == expected_kwargs, f"unexpected kwargs for {fn}: {actual}"
        assert all(name == name.lower() for name in actual), (
            f"non snake_case kwargs in {fn}: {actual}"
        )
        _assert_no_any_annotations(fn)


def test_lifecycle_methods_are_thinwedge_scoped() -> None:
    assert hasattr(ThinWedge, "thread_resume")
    assert hasattr(ThinWedge, "thread_fork")
    assert hasattr(ThinWedge, "thread_archive")
    assert hasattr(ThinWedge, "thread_unarchive")
    assert hasattr(AsyncThinWedge, "thread_resume")
    assert hasattr(AsyncThinWedge, "thread_fork")
    assert hasattr(AsyncThinWedge, "thread_archive")
    assert hasattr(AsyncThinWedge, "thread_unarchive")
    assert not hasattr(ThinWedge, "thread")
    assert not hasattr(AsyncThinWedge, "thread")

    assert not hasattr(Thread, "resume")
    assert not hasattr(Thread, "fork")
    assert not hasattr(Thread, "archive")
    assert not hasattr(Thread, "unarchive")
    assert not hasattr(AsyncThread, "resume")
    assert not hasattr(AsyncThread, "fork")
    assert not hasattr(AsyncThread, "archive")
    assert not hasattr(AsyncThread, "unarchive")

    for fn in (
        ThinWedge.thread_archive,
        ThinWedge.thread_unarchive,
        AsyncThinWedge.thread_archive,
        AsyncThinWedge.thread_unarchive,
    ):
        _assert_no_any_annotations(fn)


def test_initialize_metadata_parses_user_agent_shape() -> None:
    payload = InitializeResponse.model_validate({"userAgent": "thinwedge-cli/1.2.3"})
    parsed = ThinWedge._validate_initialize(payload)
    assert parsed is payload
    assert parsed.userAgent == "thinwedge-cli/1.2.3"
    assert parsed.serverInfo is not None
    assert parsed.serverInfo.name == "thinwedge-cli"
    assert parsed.serverInfo.version == "1.2.3"


def test_initialize_metadata_requires_non_empty_information() -> None:
    try:
        ThinWedge._validate_initialize(InitializeResponse.model_validate({}))
    except RuntimeError as exc:
        assert "missing required metadata" in str(exc)
    else:
        raise AssertionError(
            "expected RuntimeError when initialize metadata is missing"
        )
