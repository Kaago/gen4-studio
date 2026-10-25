"""A tiny MCP server for the tests: newline-delimited JSON-RPC 2.0 over stdio.

Tools: `echo` answers its arguments, `give` its argument `give`, `seen` the methods received so far,
`fail` a tool error (message: the argument `message`), `save` and `build_rom` answer like the real ones,
`noise` an answer behind stray output, `hang` never answers, `chatter` sends notifications every 0.1 s
and never answers, `crash` writes to stderr and exits with code 3; any other name is a JSON-RPC error.
FAKE_MCP_DIE_AT_START=1 exits before the handshake; FAKE_MCP_ARGV=<file> writes the command line there.
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
    elif name == "give":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": arguments.get("give")})})
    elif name == "save":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": {"saved": True}})})
    elif name == "build_rom":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": {"out": arguments.get("out")}})})
    elif name == "seen":
        send({"jsonrpc": "2.0", "id": request_id, "result": tool_result({"result": seen})})
    elif name == "fail":
        message = arguments.get("message", "Das geht so nicht.")
        error = {"code": "invalid_input", "message": message, "details": {"field": "id"}}
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
    elif name == "chatter":
        for _ in range(600):
            send({"jsonrpc": "2.0", "method": "notifications/progress", "params": {"progress": 1}})
            time.sleep(0.1)
    elif name == "crash":
        sys.stderr.write("boom one\nboom two\n")
        sys.stderr.flush()
        sys.exit(3)
    else:
        send({"jsonrpc": "2.0", "id": request_id, "error": {"code": -32602, "message": f"Unknown tool: {name}"}})


def main() -> None:
    if os.environ.get("FAKE_MCP_ARGV"):
        with open(os.environ["FAKE_MCP_ARGV"], "w", encoding="utf-8") as f:
            json.dump(sys.argv[1:], f)
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
