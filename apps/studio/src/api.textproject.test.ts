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

import { adoptTextProject, conflictDiff, entityIds, externalSync, migrateTextProject, planRenameId, renameId, resolveExternalConflict, revertTextProject, textProjectStatus } from "./api";
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

  it("set_up_take_back_lift_rename_and_ids_send_the_documented_arguments", async () => {
    // The set-up reports its phases over a channel in the app.
    const onProgress = vi.fn();
    await adoptTextProject(onProgress);
    const [cmd, args] = invoke.mock.calls[invoke.mock.calls.length - 1]!;
    expect(cmd).toBe("adopt_text_project");
    args.progress.onmessage({ phase: "entities", done: 3, total: 17 });
    expect(onProgress).toHaveBeenCalledWith({ phase: "entities", done: 3, total: 17 });
    await revertTextProject();
    expect(invoke).toHaveBeenLastCalledWith("revert_text_project");
    await migrateTextProject();
    expect(invoke).toHaveBeenLastCalledWith("migrate_text_project");
    await planRenameId("TRAINER_JENS", "TRAINER_JENS_2");
    expect(invoke).toHaveBeenLastCalledWith("plan_rename_id", { id: "TRAINER_JENS", newId: "TRAINER_JENS_2" });
    await renameId("TRAINER_JENS", "TRAINER_JENS_2");
    expect(invoke).toHaveBeenLastCalledWith("rename_id", { id: "TRAINER_JENS", newId: "TRAINER_JENS_2" });
    invoke.mockResolvedValueOnce({ "25": "SPECIES_PIKACHU" });
    const ids = await entityIds("Species");
    expect(invoke).toHaveBeenLastCalledWith("entity_ids", { kind: "Species" });
    expect(ids[25]).toBe("SPECIES_PIKACHU");
    // The dev bridge: no channel, no progress.
    inTauri.mockReturnValue(false);
    invoke.mockReset();
    invoke.mockResolvedValueOnce(null);
    await adoptTextProject(onProgress);
    expect(invoke.mock.calls).toEqual([["adopt_text_project"]]);
  });
});
