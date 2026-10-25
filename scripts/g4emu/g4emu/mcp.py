"""A small MCP client over stdio, enough to drive `studio mcp` from scenarios.

The transport is newline-delimited JSON-RPC 2.0: one JSON object per line on the server's
stdout (answers) and stdin (requests). No PyPI dependency; the handshake is `initialize`,
then the `notifications/initialized` notification, then any number of `tools/call`.
"""

from __future__ import annotations

import json
import queue
import subprocess
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import IO, Any

PROTOCOL_VERSION = "2025-11-25"
ANSWER_TIMEOUT = 600.0  # seconds to wait for one answer (a write may rebuild a lot)
HANDSHAKE_TIMEOUT = 30.0  # at least this long for `initialize` (the server opens the project first)
_CLOSE_GRACE = 5.0  # seconds a closing server may need to finish before it is killed
_STDERR_LINES = 5  # how many of the server's last stderr lines a "dead" message quotes


class McpError(Exception):
    """The server is gone, silent, or answered with a JSON-RPC error (the message is German)."""


@dataclass
class McpResult:
    """One tool call. `ok` is `not isError`; `result` is `structuredContent["result"]` on success,
    `error` the whole `structuredContent` ({code, message, details}) on a tool error."""

    ok: bool
    result: Any
    error: dict | None


class McpSession:
    """A running `studio mcp` process after the handshake. Use as a context manager."""

    def __init__(self, process: subprocess.Popen, stderr_log: IO[bytes], log: Path, timeout: float) -> None:
        self._process = process
        self._stderr_log = stderr_log
        self._log = log
        self._timeout = timeout
        self._next_id = 0
        self._lines: queue.Queue[str | None] = queue.Queue()  # None marks the end of stdout
        self._stuck = False  # an answer timed out: the server may still be busy, close() kills it
        self.closed = False
        self.server_info: dict = {}
        self.protocol_version = ""
        threading.Thread(target=self._pump, daemon=True).start()

    @classmethod
    def start(cls, command: list[str], log: Path, timeout: float = ANSWER_TIMEOUT) -> McpSession:
        """Start the server (stderr goes to `log`, whose folder must exist) and do the handshake.

        `timeout` limits each answer; the handshake waits at least `HANDSHAKE_TIMEOUT`."""
        try:
            stderr_log = open(log, "wb")
        except OSError as e:
            reason = "Ordner fehlt" if isinstance(e, FileNotFoundError) else (e.strerror or str(e))
            raise McpError(f"MCP-Protokoll nicht schreibbar: {log} ({reason})") from e
        try:
            process = subprocess.Popen(
                command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr_log,
                text=True, encoding="utf-8", bufsize=1,
            )
        except OSError as e:
            stderr_log.close()
            raise McpError(f"MCP-Server nicht gestartet ({command[0]}): {e}") from e
        session = cls(process, stderr_log, log, timeout)
        try:
            answer = session._request("initialize", {
                "protocolVersion": PROTOCOL_VERSION, "capabilities": {},
                "clientInfo": {"name": "g4emu", "version": "0"},
            }, timeout=max(timeout, HANDSHAKE_TIMEOUT))
            session.protocol_version = str(answer.get("protocolVersion", ""))
            session.server_info = dict(answer.get("serverInfo") or {})
            session._send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        except BaseException:
            session.close()
            raise
        return session

    def call(self, tool: str, arguments: dict) -> McpResult:
        """`tools/call`. A tool error comes back as `McpResult(ok=False, …)`; a JSON-RPC error raises."""
        answer = self._request("tools/call", {"name": tool, "arguments": arguments})
        structured = answer.get("structuredContent")
        if answer.get("isError"):
            if not isinstance(structured, dict):  # no structured error: keep the text
                text = " ".join(str(c.get("text", "")) for c in answer.get("content", []) if isinstance(c, dict))
                structured = {"code": "tool_error", "message": text, "details": None}
            return McpResult(ok=False, result=None, error=structured)
        result = structured.get("result") if isinstance(structured, dict) else None
        return McpResult(ok=True, result=result, error=None)

    def close(self) -> None:
        """End the session: close stdin, give the server a moment, then kill it. Safe to repeat."""
        if self.closed:
            return
        self.closed = True
        try:
            if self._process.stdin is not None:
                self._process.stdin.close()
        except OSError:
            pass
        try:
            self._process.wait(timeout=0 if self._stuck else _CLOSE_GRACE)
        except subprocess.TimeoutExpired:
            self._process.kill()
            self._process.wait()
        if self._process.stdout is not None:
            self._process.stdout.close()
        self._stderr_log.close()

    def __enter__(self) -> McpSession:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    # --- transport ---

    def _pump(self) -> None:
        """Move the server's stdout lines into the queue (a thread, so that reads can time out)."""
        try:
            for line in self._process.stdout:  # type: ignore[union-attr]
                self._lines.put(line)
        except (OSError, ValueError):  # closed while reading
            pass
        self._lines.put(None)

    def _send(self, message: dict) -> None:
        if self.closed:
            raise McpError("MCP-Sitzung ist geschlossen")
        try:
            assert self._process.stdin is not None
            self._process.stdin.write(json.dumps(message) + "\n")
            self._process.stdin.flush()
        except OSError as e:  # broken pipe: the server is gone
            raise self._dead() from e

    def _request(self, method: str, params: dict, timeout: float | None = None) -> dict:
        """Send a request and return its `result`, skipping stray lines, notifications and server requests.

        The time limit (`timeout`, else the session's) holds for the answer as a whole: lines that are
        no answer, a notification every moment say, do not extend it."""
        limit = self._timeout if timeout is None else timeout
        self._next_id += 1
        request_id = self._next_id
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        deadline = time.monotonic() + limit
        while True:
            try:
                line = self._lines.get(timeout=max(deadline - time.monotonic(), 0.0))
            except queue.Empty:
                self._stuck = True
                raise McpError(f"MCP-Server: keine Antwort auf {method} innerhalb von {limit:g} s") from None
            if line is None:
                self._lines.put(None)  # a later call finds the end again
                raise self._dead()
            try:
                message = json.loads(line)
            except ValueError:
                continue  # stray output
            if not isinstance(message, dict) or "method" in message or message.get("id") != request_id:
                continue  # a notification, a request of the server, or an answer to something else
            if "error" in message:
                error = message["error"] if isinstance(message["error"], dict) else {}
                raise McpError(f"MCP-Fehler {error.get('code')}: {error.get('message', message['error'])}")
            result = message.get("result")
            return result if isinstance(result, dict) else {}

    def _dead(self) -> McpError:
        """The error for a server that ended: its exit code and the last lines of its stderr."""
        try:
            code = self._process.wait(timeout=_CLOSE_GRACE)
        except subprocess.TimeoutExpired:
            code = "?"
        try:
            tail = self._log.read_text(encoding="utf-8", errors="replace").splitlines()[-_STDERR_LINES:]
        except OSError:
            tail = []
        return McpError(f"MCP-Server beendet (Exit {code}): {' | '.join(tail)}")
