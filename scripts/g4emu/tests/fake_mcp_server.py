"""A tiny MCP server for the tests: newline-delimited JSON-RPC 2.0 over stdio.

Tools: `echo` answers its arguments, `seen` the methods received so far, `fail` a tool error,
`unknown` a JSON-RPC error, `noise` an answer behind stray output, `hang` never answers,
`crash` writes to stderr and exits with code 3. FAKE_MCP_DIE_AT_START=1 exits before the handshake.
"""

import json
import os
import sys
import time

seen: list[str] = []


def send(message: dict) -> None:
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def tool_result(content: object, is_error: bool = False) -> dict:
    return {
        "content": [{"type": "text", "text": json.dumps(content)}],
        "structuredContent": content,
        "isError": is_error,
    }


def call(request_id: int, name: str, arguments: dict) -> None:
    if name == "echo":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": arguments})})
    elif name == "seen":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": seen})})
    elif name == "fail":
        error = {"code": "invalid_input", "message": "Das geht so nicht.", "details": {"field": "id"}}
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result(error, is_error=True)})
    elif name == "noise":
        sys.stdout.write("not json at all\n")
        sys.stdout.flush()
        send({"jsonrpc": "2.0", "method": "notifications/message", "params": {"data": "hello"}})
        send({"jsonrpc": "2.0", "id": 99, "method": "ping"})  # a request of the server
        send({"jsonrpc": "2.0", "id": request_id + 1000, "result": tool_result({"result": "other"})})
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": "late"})})
    elif name == "hang":
        time.sleep(60)
    elif name == "crash":
        sys.stderr.write("boom one\nboom two\n")
        sys.stderr.flush()
        sys.exit(3)
    else:
        send({"jsonrpc": "2.0", "id": request_id, "error": {"code": -32602, "message": f"Unknown tool: {name}"}})


def main() -> None:
    if os.environ.get("FAKE_MCP_DIE_AT_START"):
        sys.stderr.write("cannot start\n")
        sys.exit(2)
    for line in sys.stdin:
        message = json.loads(line)
        method = message.get("method")
        seen.append(method)
        if method == "initialize":
            version = message["params"]["protocolVersion"]
            send({
                "jsonrpc": "2.0", "id": message["id"],
                "result": {"protocolVersion": version, "capabilities": {"tools": {}},
                           "serverInfo": {"name": "gen4-studio", "version": "0"}},
            })
        elif method == "tools/call":
            params = message["params"]
            call(message["id"], params["name"], params.get("arguments", {}))


main()
