"""The MCP client over stdio, against a small fake server (tests/fake_mcp_server.py)."""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import pytest

from g4emu import mcp, scenario
from g4emu.mcp import McpError, McpResult, McpSession
from g4emu.scenario import studio_command

FAKE = Path(__file__).parent / "fake_mcp_server.py"


def fake_command() -> list[str]:
    return [sys.executable, str(FAKE)]


def test_initialize_and_call(tmp_path):
    with McpSession.start(fake_command(), tmp_path / "mcp.log") as session:
        result = session.call("echo", {"id": 7, "name": "Route 1"})
        assert result == McpResult(ok=True, result={"id": 7, "name": "Route 1"}, error=None)
        # the handshake: initialize, then the initialized notification, then the call itself
        assert session.call("seen", {}).result == ["initialize", "notifications/initialized", "tools/call", "tools/call"]
    assert session.closed


def test_initialize_asks_for_the_protocol_version_2025_11_25(tmp_path):
    assert mcp.PROTOCOL_VERSION == "2025-11-25"
    with McpSession.start(fake_command(), tmp_path / "mcp.log") as session:
        assert session.server_info["name"] == "gen4-studio"
        assert session.protocol_version == "2025-11-25"


def test_tool_error_is_not_ok(tmp_path):
    with McpSession.start(fake_command(), tmp_path / "mcp.log") as session:
        result = session.call("fail", {})
    assert result.ok is False
    assert result.result is None
    assert result.error == {"code": "invalid_input", "message": "Das geht so nicht.", "details": {"field": "id"}}


def test_protocol_error_raises(tmp_path):
    with McpSession.start(fake_command(), tmp_path / "mcp.log") as session:
        with pytest.raises(McpError, match=r"-32602.*Unknown tool: nope"):
            session.call("nope", {})
        # the session is still usable afterwards
        assert session.call("echo", {"a": 1}).ok


def test_client_skips_stray_lines_notifications_and_other_ids(tmp_path):
    with McpSession.start(fake_command(), tmp_path / "mcp.log") as session:
        assert session.call("noise", {}).result == "late"


def test_client_reports_a_dead_server(tmp_path):
    log = tmp_path / "mcp.log"
    with McpSession.start(fake_command(), log) as session:
        with pytest.raises(McpError, match=r"MCP-Server beendet \(Exit 3\): .*boom two"):
            session.call("crash", {})
        with pytest.raises(McpError, match="beendet"):  # and stays dead
            session.call("echo", {})
    assert "boom one" in log.read_text()


def test_client_reports_a_server_that_dies_during_the_handshake(tmp_path, monkeypatch):
    monkeypatch.setenv("FAKE_MCP_DIE_AT_START", "1")
    with pytest.raises(McpError, match=r"MCP-Server beendet \(Exit 2\): cannot start"):
        McpSession.start(fake_command(), tmp_path / "mcp.log")


def test_client_gives_up_on_a_silent_server(tmp_path):
    with McpSession.start(fake_command(), tmp_path / "mcp.log", timeout=0.5) as session:
        with pytest.raises(McpError, match="keine Antwort"):
            session.call("hang", {})
    assert session.closed  # close() killed the hanging server


def test_the_time_limit_holds_per_answer_not_per_line(tmp_path):
    # notifications every 0.1 s must not keep a call waiting past its limit
    with McpSession.start(fake_command(), tmp_path / "mcp.log", timeout=1.0) as session:
        began = time.monotonic()
        with pytest.raises(McpError, match="keine Antwort auf tools/call innerhalb von 1 s"):
            session.call("chatter", {})
        assert time.monotonic() - began < 5.0
    assert session.closed


def test_the_handshake_has_a_limit_of_its_own(tmp_path):
    # a short limit for the answers does not cut the handshake short (a server under load starts slowly)
    assert mcp.HANDSHAKE_TIMEOUT >= 30.0
    with McpSession.start(fake_command(), tmp_path / "mcp.log", timeout=0.01) as session:
        assert session.server_info["name"] == "gen4-studio"


def test_a_log_in_a_missing_folder_is_an_mcp_error(tmp_path):
    with pytest.raises(McpError, match=r"MCP-Protokoll nicht schreibbar: .*fehlt"):
        McpSession.start(fake_command(), tmp_path / "fehlt" / "mcp.log")
    assert not (tmp_path / "fehlt").exists()


def test_a_missing_binary_is_an_mcp_error(tmp_path):
    with pytest.raises(McpError, match="nicht gestartet"):
        McpSession.start([str(tmp_path / "does-not-exist")], tmp_path / "mcp.log")


def test_close_is_idempotent_and_ends_the_process(tmp_path):
    session = McpSession.start(fake_command(), tmp_path / "mcp.log")
    session.close()
    session.close()
    assert session.closed
    with pytest.raises(McpError):
        session.call("echo", {})


def test_studio_command_prefers_env(tmp_path, monkeypatch):
    monkeypatch.setenv("G4_STUDIO_BIN", "/x/studio")
    assert studio_command(tmp_path) == ["/x/studio"]


def test_studio_command_builds_the_product_binary_without_env(tmp_path, monkeypatch):
    monkeypatch.delenv("G4_STUDIO_BIN", raising=False)
    calls = []

    def fake_run(cmd, **kw):
        calls.append((cmd, kw.get("cwd")))

        class R:
            returncode = 0

        return R()

    monkeypatch.setattr(scenario.subprocess, "run", fake_run)
    assert studio_command(tmp_path) == [str(tmp_path / "target" / "debug" / "studio")]
    assert calls == [(["cargo", "build", "-p", "studio", "--bin", "studio"], tmp_path)]
