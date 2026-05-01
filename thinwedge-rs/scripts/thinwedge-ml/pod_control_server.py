#!/usr/bin/env python3
import io
import json
import os
import pathlib
import shutil
import subprocess
import tarfile
import urllib.parse
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


WORKSPACE_ROOT = pathlib.Path(
    os.environ.get("THINWEDGE_CONTROL_WORKSPACE_ROOT") or "/workspace"
).resolve()
CONTROL_TOKEN = os.environ.get("THINWEDGE_CONTROL_TOKEN") or ""
CONTROL_PORT = int(os.environ.get("THINWEDGE_CONTROL_PORT") or "8000")


def resolve_workspace_path(raw_path: str) -> pathlib.Path:
    candidate = pathlib.Path(raw_path)
    if not candidate.is_absolute():
        candidate = WORKSPACE_ROOT / candidate
    resolved = candidate.resolve()
    if WORKSPACE_ROOT not in resolved.parents and resolved != WORKSPACE_ROOT:
        raise ValueError(f"path escapes workspace root: {raw_path}")
    return resolved


def safe_extract_tar(archive_bytes: bytes, destination: pathlib.Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
        for member in archive.getmembers():
            target_path = (destination / member.name).resolve()
            if destination not in target_path.parents and target_path != destination:
                raise ValueError(f"tar entry escapes destination: {member.name}")
        archive.extractall(destination)


class ThinWedgeControlHandler(BaseHTTPRequestHandler):
    server_version = "ThinWedgeControl/0.1"

    def _authorized(self) -> bool:
        if not CONTROL_TOKEN:
            return True
        return self.headers.get("X-ThinWedge-Token") == CONTROL_TOKEN

    def _read_body(self) -> bytes:
        length = int(self.headers.get("Content-Length") or "0")
        return self.rfile.read(length)

    def _json_response(self, payload: dict, status: HTTPStatus = HTTPStatus.OK) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _text_response(
        self,
        body: bytes,
        content_type: str = "application/octet-stream",
        status: HTTPStatus = HTTPStatus.OK,
    ) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _reject_if_unauthorized(self) -> bool:
        if self._authorized():
            return False
        self._json_response({"error": "unauthorized"}, HTTPStatus.UNAUTHORIZED)
        return True

    def do_GET(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path == "/health":
            self._json_response({"status": "ok"})
            return
        if self._reject_if_unauthorized():
            return
        if parsed.path == "/thinwedge/file":
            query = urllib.parse.parse_qs(parsed.query)
            raw_path = (query.get("path") or [""])[0]
            try:
                target = resolve_workspace_path(raw_path)
            except ValueError as error:
                self._json_response({"error": str(error)}, HTTPStatus.BAD_REQUEST)
                return
            if not target.is_file():
                self._json_response({"error": "file not found"}, HTTPStatus.NOT_FOUND)
                return
            self._text_response(target.read_bytes())
            return
        self._json_response({"error": "not found"}, HTTPStatus.NOT_FOUND)

    def do_POST(self) -> None:  # noqa: N802
        if self._reject_if_unauthorized():
            return
        parsed = urllib.parse.urlparse(self.path)

        if parsed.path == "/thinwedge/repository":
            destination_header = self.headers.get("X-ThinWedge-Destination") or ""
            try:
                destination = resolve_workspace_path(destination_header)
                safe_extract_tar(self._read_body(), destination)
            except (ValueError, tarfile.TarError) as error:
                self._json_response({"error": str(error)}, HTTPStatus.BAD_REQUEST)
                return
            self._json_response({"status": "ok", "destination": str(destination)})
            return

        if parsed.path == "/thinwedge/file":
            raw_path = self.headers.get("X-ThinWedge-Path") or ""
            try:
                target = resolve_workspace_path(raw_path)
            except ValueError as error:
                self._json_response({"error": str(error)}, HTTPStatus.BAD_REQUEST)
                return
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(self._read_body())
            self._json_response({"status": "ok", "path": str(target)})
            return

        if parsed.path == "/thinwedge/exec":
            try:
                request = json.loads(self._read_body().decode("utf-8") or "{}")
                command = request["command"]
                cwd = resolve_workspace_path(request.get("cwd") or str(WORKSPACE_ROOT))
                env = os.environ.copy()
                env.update(request.get("env") or {})
                timeout = request.get("timeoutSec")
            except (KeyError, ValueError, json.JSONDecodeError) as error:
                self._json_response({"error": str(error)}, HTTPStatus.BAD_REQUEST)
                return
            try:
                completed = subprocess.run(
                    command,
                    shell=True,
                    cwd=str(cwd),
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=timeout,
                    check=False,
                )
            except subprocess.TimeoutExpired as error:
                self._json_response(
                    {
                        "status": "timeout",
                        "command": command,
                        "stdout": error.stdout or "",
                        "stderr": error.stderr or "",
                    },
                    HTTPStatus.REQUEST_TIMEOUT,
                )
                return
            self._json_response(
                {
                    "status": "ok" if completed.returncode == 0 else "failed",
                    "command": command,
                    "cwd": str(cwd),
                    "exitCode": completed.returncode,
                    "stdout": completed.stdout,
                    "stderr": completed.stderr,
                },
                HTTPStatus.OK if completed.returncode == 0 else HTTPStatus.BAD_REQUEST,
            )
            return

        self._json_response({"error": "not found"}, HTTPStatus.NOT_FOUND)

    def log_message(self, fmt: str, *args) -> None:
        print(
            json.dumps(
                {
                    "remote": self.address_string(),
                    "request": self.requestline,
                    "message": fmt % args,
                }
            ),
            flush=True,
        )


if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", CONTROL_PORT), ThinWedgeControlHandler)
    print(
        json.dumps(
            {
                "status": "starting",
                "port": CONTROL_PORT,
                "workspaceRoot": str(WORKSPACE_ROOT),
            }
        ),
        flush=True,
    )
    server.serve_forever()
