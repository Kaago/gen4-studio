#!/usr/bin/env bash
# MCP client of the agent test environment (see mcp_client.py):
#   scripts/agent/mcp.sh tools | call <tool> ['<json>'] | read <uri> | close
exec python3 "$(dirname "$0")/mcp_client.py" "$@"
