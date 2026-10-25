import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import {
  agentAnswer,
  agentDisconnect,
  agentPoll,
  agentRemoveGrant,
  agentSetEnabled,
  agentSetExpert,
  agentSetGrant,
  agentSetReadOnly,
  agentSettings,
  agentStatus,
} from "./api";
import type { AgentPoll, AgentSettingsDto } from "./api";

/** A poll as `agent_poll` answers it (see `src-tauri/src/mcp/hub.rs`). */
const poll: AgentPoll = {
  seq: 2,
  events: [
    { kind: "changed", seq: 1, client: "Claude Code", tool: "set_header", label: "Header 3 geändert", assets: [{ kind: "Event", index: 3 }] },
    { kind: "status", seq: 2 },
  ],
  status: {
    enabled: true,
    expert: false,
    read_only: false,
    port: 49152,
    bridge: false,
    connections: [{ id: 1, client: "Claude Code", since_ms: 1_761_400_000_000 }],
    question: { id: 4, kind: "connect", client: "Claude Code", project: "platin", what: null },
  },
};

describe("agent commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("reads the status, the events after a seq and the settings", async () => {
    invoke.mockResolvedValue(poll);
    await expect(agentPoll(7)).resolves.toEqual(poll);
    expect(invoke).toHaveBeenLastCalledWith("agent_poll", { since: 7 });
    await agentStatus();
    expect(invoke).toHaveBeenLastCalledWith("agent_status");
    const settings: AgentSettingsDto = {
      settings: { version: 1, enabled: true, expert: false, grants: [{ path: "/p/platin", base_id: null, level: "read" }] },
      setup_line: 'claude mcp add gen4-studio -- "/Applications/Gen4 Studio.app/Contents/MacOS/studio" mcp',
      open_project: { path: "/p/platin", level: null },
    };
    invoke.mockResolvedValue(settings);
    await expect(agentSettings()).resolves.toEqual(settings);
    expect(invoke).toHaveBeenLastCalledWith("agent_settings");
  });

  it("passes the switches and the grant with the keys the commands take", async () => {
    await agentSetEnabled(true);
    expect(invoke).toHaveBeenLastCalledWith("agent_set_enabled", { on: true });
    await agentSetExpert(false);
    expect(invoke).toHaveBeenLastCalledWith("agent_set_expert", { on: false });
    await agentSetReadOnly(true);
    expect(invoke).toHaveBeenLastCalledWith("agent_set_read_only", { on: true });
    await agentSetGrant("write");
    expect(invoke).toHaveBeenLastCalledWith("agent_set_grant", { level: "write" });
    await agentRemoveGrant("/p/platin");
    expect(invoke).toHaveBeenLastCalledWith("agent_remove_grant", { path: "/p/platin" });
  });

  it("answers a question by id and disconnects without arguments", async () => {
    await agentAnswer(4, "read");
    expect(invoke).toHaveBeenLastCalledWith("agent_answer", { question: 4, answer: "read" });
    await agentDisconnect();
    expect(invoke).toHaveBeenLastCalledWith("agent_disconnect");
  });
});
