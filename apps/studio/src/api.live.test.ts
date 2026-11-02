import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { liveBreakpointClear, liveBreakpoints, liveBreakpointSet, liveConnect, livePlay, liveResume, liveSet, liveStatus, liveStop } from "./api";

describe("live commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  // The keys are the ones `every_command_accepts_the_keys_api_ts_sends` (dispatch.rs) sends to the bridge.
  it("live_commands_send_the_keys_the_bridge_accepts", async () => {
    await liveStatus();
    expect(invoke).toHaveBeenLastCalledWith("live_status", {});
    await livePlay("/Applications/melonDS.app", "/tmp/rom.nds");
    expect(invoke).toHaveBeenLastCalledWith("live_play", { emulator: "/Applications/melonDS.app", rom: "/tmp/rom.nds" });
    await liveConnect(3390);
    expect(invoke).toHaveBeenLastCalledWith("live_connect", { port: 3390, host: null });
    await liveConnect(3333, "192.168.0.2");
    expect(invoke).toHaveBeenLastCalledWith("live_connect", { port: 3333, host: "192.168.0.2" });
    await liveStop();
    expect(invoke).toHaveBeenLastCalledWith("live_stop", {});
    await liveResume();
    expect(invoke).toHaveBeenLastCalledWith("live_resume", {});
    await liveSet("flag", 0x173, 0);
    expect(invoke).toHaveBeenLastCalledWith("live_set", { kind: "flag", number: 0x173, value: 0 });
    await liveSet("var", 0x40a5, 1);
    expect(invoke).toHaveBeenLastCalledWith("live_set", { kind: "var", number: 0x40a5, value: 1 });
    await liveBreakpoints();
    expect(invoke).toHaveBeenLastCalledWith("live_breakpoints", {});
    // Tauri maps camelCase to the command's snake_case parameters; the bridge takes both.
    await liveBreakpointSet({ event_file: 12, kind: "coord", index: 2 });
    expect(invoke).toHaveBeenLastCalledWith("live_breakpoint_set", { eventFile: 12, kind: "coord", index: 2 });
    await liveBreakpointClear(4);
    expect(invoke).toHaveBeenLastCalledWith("live_breakpoint_clear", { id: 4 });
  });
});
