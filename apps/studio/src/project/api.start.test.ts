import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const inTauri = vi.hoisted(() => vi.fn(() => true));
vi.mock("../platform/transport", () => ({ invoke, inTauri }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import { checkProjectDest, createProject, defaultProjectParent, inspectRom } from "../api";
import type { Created, DestCheck } from "../api";

const created: Created = {
  root: "/tmp/neu",
  game_code: "CPUD",
  template: "original",
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  phases: [
    { phase: "read", ms: 30 },
    { phase: "extract", ms: 900 },
    { phase: "template", ms: 0 },
    { phase: "check", ms: 40 },
  ],
};

describe("api.start", () => {
  beforeEach(() => {
    invoke.mockReset();
    inTauri.mockReset();
    inTauri.mockReturnValue(true);
  });

  it("calls_each_command_with_the_argument_names_of_the_backend", async () => {
    invoke.mockResolvedValue(undefined);
    await inspectRom("/roms/pt.nds");
    await defaultProjectParent();
    await checkProjectDest("/p/neu", 134217728);
    expect(invoke.mock.calls).toEqual([
      ["inspect_rom", { path: "/roms/pt.nds" }],
      ["default_project_parent"],
      ["check_project_dest", { dest: "/p/neu", romSize: 134217728 }],
    ]);
  });

  it("DestCheck_has_a_state_for_a_path_that_cannot_be_read", () => {
    const check: DestCheck = { state: "unreadable", free_bytes: null, needed_bytes: 1, ok: false };
    expect(check.ok).toBe(false);
  });

  it("createProject_passes_a_channel_that_feeds_the_progress_callback_and_resolves_to_the_result", async () => {
    invoke.mockResolvedValue(created);
    const onProgress = vi.fn();
    await expect(createProject("/roms/pt.nds", "/p/neu", "data_only", onProgress)).resolves.toBe(created);
    const [cmd, args] = invoke.mock.calls[0]!;
    expect(cmd).toBe("create_project");
    expect(args).toMatchObject({ rom: "/roms/pt.nds", dest: "/p/neu", template: "data_only" });
    args.progress.onmessage({ phase: "extract", done: 0, total: 1 });
    expect(onProgress).toHaveBeenCalledWith({ phase: "extract", done: 0, total: 1 });
  });

  it("createProject_outside_tauri_calls_the_bridge_without_a_channel_and_reports_the_four_phases_done", async () => {
    inTauri.mockReturnValue(false);
    invoke.mockResolvedValue(created);
    const onProgress = vi.fn();
    await expect(createProject("/roms/pt.nds", "/p/neu", "original", onProgress)).resolves.toBe(created);
    expect(invoke.mock.calls).toEqual([["create_project", { rom: "/roms/pt.nds", dest: "/p/neu", template: "original" }]]);
    expect(onProgress.mock.calls.map(([p]) => p)).toEqual([
      { phase: "read", done: 0, total: 0 },
      { phase: "read", done: 1, total: 1 },
      { phase: "extract", done: 1, total: 1 },
      { phase: "template", done: 1, total: 1 },
      { phase: "check", done: 1, total: 1 },
    ]);
  });

  it("createProject_outside_tauri_reports_the_first_phase_running_while_the_bridge_works", async () => {
    // Without a channel the bridge answers only at the end (seconds): until then the first phase runs, so the wizard
    // does not show every phase as waiting while the work goes on.
    inTauri.mockReturnValue(false);
    let answer: (value: Created) => void = () => {};
    invoke.mockReturnValue(new Promise<Created>((resolve) => (answer = resolve)));
    const onProgress = vi.fn();
    const pending = createProject("/roms/pt.nds", "/p/neu", "original", onProgress);
    expect(onProgress.mock.calls.map(([p]) => p)).toEqual([{ phase: "read", done: 0, total: 0 }]);
    answer(created);
    await expect(pending).resolves.toBe(created);
  });

  it("createProject_reports_only_the_start_when_the_command_fails", async () => {
    inTauri.mockReturnValue(false);
    invoke.mockRejectedValue({ code: "error", message: "destination is not empty", asset: null });
    const onProgress = vi.fn();
    await expect(createProject("/r.nds", "/d", "original", onProgress)).rejects.toMatchObject({ message: "destination is not empty" });
    expect(onProgress.mock.calls.map(([p]) => p)).toEqual([{ phase: "read", done: 0, total: 0 }]);
  });
});
