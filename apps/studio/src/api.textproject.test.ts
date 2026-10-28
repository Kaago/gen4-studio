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

import { conflictDiff, externalSync, resolveExternalConflict, textProjectStatus } from "./api";
import type { SyncOutcome } from "./api";

const outcome: SyncOutcome = { adopted: [], applied: [], conflicts: [], broken: [], reloaded: [], full: true, ms: 40 };

describe("text project commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    inTauri.mockReturnValue(true);
  });

  it("commands_send_the_documented_arguments", async () => {
    await textProjectStatus();
    expect(invoke).toHaveBeenLastCalledWith("text_project_status");
    await conflictDiff("species/0025");
    expect(invoke).toHaveBeenLastCalledWith("conflict_diff", { key: "species/0025" });
    await resolveExternalConflict("species/0025", "dspre");
    expect(invoke).toHaveBeenLastCalledWith("resolve_external_conflict", { key: "species/0025", keep: "dspre" });
    await resolveExternalConflict("moves/0001", "studio");
    expect(invoke).toHaveBeenLastCalledWith("resolve_external_conflict", { key: "moves/0001", keep: "studio" });

    // In the app the progress comes over a channel.
    invoke.mockResolvedValueOnce(outcome);
    const onProgress = vi.fn();
    await expect(externalSync(onProgress)).resolves.toBe(outcome);
    const [cmd, args] = invoke.mock.calls[invoke.mock.calls.length - 1]!;
    expect(cmd).toBe("external_sync");
    args.progress.onmessage({ done: 10, total: 400 });
    expect(onProgress).toHaveBeenCalledWith({ done: 10, total: 400 });

    // The dev bridge has no channel: the call runs without one and reports no progress.
    inTauri.mockReturnValue(false);
    invoke.mockReset();
    invoke.mockResolvedValueOnce(outcome);
    const later = vi.fn();
    await expect(externalSync(later)).resolves.toBe(outcome);
    expect(invoke.mock.calls).toEqual([["external_sync"]]);
    expect(later).not.toHaveBeenCalled();
  });
});
