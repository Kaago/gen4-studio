import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const inTauri = vi.hoisted(() => vi.fn(() => true));
vi.mock("./platform/transport", () => ({ invoke, inTauri }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import { gitInit, gitStatus, isCmdError, restoreTextProject } from "./api";

/** The arguments of the last call. */
const last = () => invoke.mock.calls[invoke.mock.calls.length - 1]!;

describe("git commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    inTauri.mockReturnValue(true);
  });

  it("git_commands_send_the_documented_arguments", async () => {
    await gitStatus();
    expect(invoke).toHaveBeenLastCalledWith("git_status");

    // „Git einrichten“: the option in Tauri's camelCase, the phases over a channel.
    const onInit = vi.fn();
    await gitInit({}, onInit);
    let [cmd, args] = last();
    expect(cmd).toBe("git_init");
    expect(args.templatesOnly).toBe(false);
    args.progress.onmessage({ phase: "add" });
    expect(onInit).toHaveBeenCalledWith({ phase: "add" });
    await gitInit({ templatesOnly: true }, onInit);
    expect(last()[1].templatesOnly).toBe(true);

    // Completing a clone: the folder, the chosen base ROM or null, the phases over a channel.
    const onRestore = vi.fn();
    await restoreTextProject("/k/klon", null, onRestore);
    [cmd, args] = last();
    expect(cmd).toBe("restore_text_project");
    expect(args).toMatchObject({ root: "/k/klon", nds: null });
    args.progress.onmessage({ phase: "import", done: 3, total: 17 });
    expect(onRestore).toHaveBeenCalledWith({ phase: "import", done: 3, total: 17 });
    await restoreTextProject("/k/klon", "/roms/pt.nds", onRestore);
    expect(last()[1]).toMatchObject({ root: "/k/klon", nds: "/roms/pt.nds" });

    // The dev bridge has no channel: the calls run without one and report no progress.
    inTauri.mockReturnValue(false);
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    await gitInit({ templatesOnly: true }, onInit);
    await restoreTextProject("/k/klon", null, onRestore);
    expect(invoke.mock.calls).toEqual([
      ["git_init", { templatesOnly: true }],
      ["restore_text_project", { root: "/k/klon", nds: null }],
    ]);
  });

  it("git_refusals_are_command_errors", () => {
    for (const code of ["restore_needed", "no_repo", "no_commit", "not_diffable", "unsaved", "busy"]) {
      expect(isCmdError({ code, message: "Satz.", asset: null })).toBe(true);
    }
  });
});
