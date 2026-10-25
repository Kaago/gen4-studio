#!/usr/bin/env python3
"""MCP client of the agent test environment: talks to `/mcp` on the running dev bridge.

    scripts/agent/mcp.sh tools                     tool names, one per line
    scripts/agent/mcp.sh call <tool> ['<json>']    structuredContent as compact JSON
    scripts/agent/mcp.sh read <uri>                the resource's text
    scripts/agent/mcp.sh close                     ends the session

Exit codes: 0 ok, 1 the tool failed (its structuredContent goes to stderr), 2 usage,
protocol or HTTP error (message on stderr).

The session id lives in target/agent/mcp.session; when the bridge no longer knows it (HTTP
404, e.g. after a restart or "Trennen") the client connects again once and retries. Port
and token come from the files studio-web.sh writes, as for call.sh. Standard library only.
"""

import json
import os
import sys
import urllib.error
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
AGENT_DIR = os.path.join(ROOT, "target", "agent")
TOKEN_FILE = os.path.join(AGENT_DIR, "bridge.token")
PORT_FILE = os.path.join(AGENT_DIR, "bridge.port")
SESSION_FILE = os.path.join(AGENT_DIR, "mcp.session")

PROTOCOL = "2025-11-25"
CLIENT = {"name": "agent-gate", "version": "1"}
# A question to the user may wait 120 s, build_rom takes a while.
TIMEOUT = 600
USAGE = "Aufruf: scripts/agent/mcp.sh tools | call <werkzeug> ['<json>'] | read <uri> | close"


class Failure(Exception):
    """A protocol or HTTP error: message on stderr, exit 2."""


class SessionGone(Exception):
    """The bridge answered 404 for the stored session."""


def read_file(path):
    with open(path, encoding="utf-8") as f:
        return f.read().strip()


def endpoint():
    try:
        token = read_file(TOKEN_FILE)
    except OSError:
        raise Failure(f"Token-Datei fehlt ({TOKEN_FILE}) - läuft scripts/agent/studio-web.sh?")
    try:
        port = read_file(PORT_FILE) or "1431"
    except OSError:
        port = "1431"
    return port, token


# No proxies: the bridge is on loopback (call.sh uses --noproxy '*' for the same reason).
OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def http(method, payload, session):
    """One request to /mcp; answers (status, headers, body text)."""
    port, token = endpoint()
    headers = {
        "X-G4-Bridge-Token": token,
        "Accept": "application/json, text/event-stream",
        "Content-Type": "application/json",
    }
    if session:
        headers["Mcp-Session-Id"] = session
        headers["MCP-Protocol-Version"] = PROTOCOL
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/mcp", data=data, headers=headers, method=method
    )
    try:
        with OPENER.open(request, timeout=TIMEOUT) as response:
            return response.status, response.headers, response.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, e.headers, e.read().decode("utf-8", "replace")
    except TimeoutError:
        raise Failure(f"Keine Antwort in {TIMEOUT} s (127.0.0.1:{port})")
    except urllib.error.URLError as e:
        if isinstance(e.reason, TimeoutError):
            raise Failure(f"Keine Antwort in {TIMEOUT} s (127.0.0.1:{port})")
        raise Failure(f"Brücke nicht erreichbar (127.0.0.1:{port}): {e} - läuft scripts/agent/studio-web.sh?")
    except OSError as e:
        raise Failure(f"Brücke nicht erreichbar (127.0.0.1:{port}): {e} - läuft scripts/agent/studio-web.sh?")


def messages(headers, body):
    """The JSON-RPC messages of an answer: plain JSON or SSE `data:` lines."""
    content_type = (headers.get("Content-Type") or "") if headers else ""
    if "text/event-stream" in content_type or body.lstrip().startswith(("data:", "event:", "id:")):
        found, data = [], []
        for line in body.splitlines() + [""]:
            if line.startswith("data:"):
                data.append(line[5:].lstrip())
            elif line == "" and data:
                text = "\n".join(data).strip()
                data = []
                if text:
                    found.append(json.loads(text))
        return found
    body = body.strip()
    if not body:
        return []
    parsed = json.loads(body)
    return parsed if isinstance(parsed, list) else [parsed]


def refusal(status, body):
    """The message of a refused request (the bridge answers a CmdError)."""
    try:
        message = json.loads(body).get("message")
    except (ValueError, AttributeError):
        message = None
    return f"HTTP {status}: {message or body.strip() or 'ohne Antworttext'}"


def rpc(method, params, session, ident):
    """A JSON-RPC request; answers its result. HTTP 404 with a session: SessionGone."""
    payload = {"jsonrpc": "2.0", "id": ident, "method": method}
    if params is not None:
        payload["params"] = params
    status, headers, body = http("POST", payload, session)
    if status == 404 and session:
        raise SessionGone()
    if status != 200:
        raise Failure(refusal(status, body))
    try:
        found = messages(headers, body)
    except ValueError as e:
        raise Failure(f"Antwort ist kein JSON: {e}")
    for message in found:
        if message.get("id") != ident:
            continue
        if "error" in message:
            error = message["error"]
            raise Failure(f"MCP-Fehler {error.get('code')}: {error.get('message')}")
        return message.get("result", {})
    raise Failure(f"Keine Antwort auf {method}")


def initialize():
    """A new session: initialize, notifications/initialized; stores and answers its id."""
    payload = {
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {"protocolVersion": PROTOCOL, "capabilities": {}, "clientInfo": CLIENT},
    }
    status, headers, body = http("POST", payload, None)
    if status != 200:
        raise Failure(refusal(status, body))
    session = headers.get("Mcp-Session-Id")
    if not session:
        raise Failure("initialize ohne Mcp-Session-Id")
    for message in messages(headers, body):
        if "error" in message:
            raise Failure(f"initialize abgelehnt: {message['error'].get('message')}")
    note = {"jsonrpc": "2.0", "method": "notifications/initialized"}
    status, _, body = http("POST", note, session)
    if status not in (200, 202):
        raise Failure(refusal(status, body))
    os.makedirs(AGENT_DIR, exist_ok=True)
    fd = os.open(SESSION_FILE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(session)
    return session


def stored_session():
    try:
        return read_file(SESSION_FILE) or None
    except OSError:
        return None


def request(method, params=None):
    """A request in the stored session (a new one if there is none or it is gone)."""
    session = stored_session()
    fresh = session is None
    if fresh:
        session = initialize()
    try:
        return rpc(method, params, session, 1)
    except SessionGone:
        if fresh:
            raise Failure("Sitzung sofort verloren (HTTP 404)")
    session = initialize()
    try:
        return rpc(method, params, session, 1)
    except SessionGone:
        raise Failure("Sitzung sofort verloren (HTTP 404)")


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def tools():
    names, cursor = [], None
    while True:
        result = request("tools/list", {"cursor": cursor} if cursor else None)
        names += [tool["name"] for tool in result.get("tools", [])]
        cursor = result.get("nextCursor")
        if not cursor:
            break
    for name in names:
        print(name)
    return 0


def call(name, arguments):
    result = request("tools/call", {"name": name, "arguments": arguments})
    if "structuredContent" in result:
        out = compact(result["structuredContent"])
    else:
        out = "\n".join(c.get("text", "") for c in result.get("content", []) if c.get("type") == "text")
    if result.get("isError"):
        print(out, file=sys.stderr)
        return 1
    print(out)
    return 0


def read(uri):
    result = request("resources/read", {"uri": uri})
    for content in result.get("contents", []):
        if "text" in content:
            print(content["text"])
        elif "blob" in content:
            print(f"<{len(content['blob'])} Zeichen base64, {content.get('mimeType', '?')}>")
    return 0


def close():
    session = stored_session()
    if session:
        status, _, body = http("DELETE", None, session)
        if status not in (200, 202, 204, 404):
            print(refusal(status, body), file=sys.stderr)
    try:
        os.remove(SESSION_FILE)
    except FileNotFoundError:
        pass
    return 0


def main(argv):
    if not argv:
        print(USAGE, file=sys.stderr)
        return 2
    command, rest = argv[0], argv[1:]
    if command == "tools" and not rest:
        return tools()
    if command == "call" and 1 <= len(rest) <= 2:
        try:
            arguments = json.loads(rest[1]) if len(rest) == 2 and rest[1].strip() else {}
        except ValueError as e:
            print(f"Argumente sind kein JSON: {e}", file=sys.stderr)
            return 2
        return call(rest[0], arguments)
    if command == "read" and len(rest) == 1:
        return read(rest[0])
    if command == "close" and not rest:
        return close()
    print(USAGE, file=sys.stderr)
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except Failure as e:
        print(e, file=sys.stderr)
        sys.exit(2)
