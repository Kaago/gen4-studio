import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { deleteEntity, deleteTargetLabel, deleteWritesFiles, isCmdError, planDelete, restoreEntity, tombstoneOf } from "./api";
import type { DeletePlan, DeleteResult, DeleteTarget, RefDto } from "./api";

const TRAINER: DeleteTarget = { kind: "trainer", index: 901 };
const REF: RefDto = {
  source: { kind: "Event", index: 8 },
  field: "script",
  slot: 2,
  label: "Figur 2 in Eventdatei 0008",
  source_name: "Eventdatei 0008",
  nav: { kind: "event", eventFile: 8, eventKind: "object", index: 2 },
};
const PLAN: DeletePlan = {
  target: TRAINER,
  label: "Trainer 0901",
  refusal: null,
  references: [],
  warnings: [],
  original: false,
  deleted: false,
  can_free: { count: 1, files: ["data/trainers/0901.json"] },
  cannot_free: null,
};
const RESULT: DeleteResult = { change: "c12", changed: [{ kind: "Trainer", index: 901 }], freed: null, ms: 4 };

describe("delete commands", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("plan_delete_entity_and_restore_send_the_documented_arguments", async () => {
    invoke.mockResolvedValueOnce(PLAN);
    await expect(planDelete(TRAINER)).resolves.toBe(PLAN);
    expect(invoke).toHaveBeenLastCalledWith("plan_delete", { target: TRAINER });

    invoke.mockResolvedValueOnce(RESULT);
    await expect(deleteEntity(TRAINER, false)).resolves.toBe(RESULT);
    expect(invoke).toHaveBeenLastCalledWith("delete_entity", { target: TRAINER, freeNumber: false });
    invoke.mockResolvedValueOnce({ ...RESULT, change: null, freed: { count: 1, files: ["data/trainers/0901.json"] } });
    await deleteEntity({ kind: "script", file: 1200 }, true);
    expect(invoke).toHaveBeenLastCalledWith("delete_entity", { target: { kind: "script", file: 1200 }, freeNumber: true });

    invoke.mockResolvedValueOnce(RESULT);
    await expect(restoreEntity({ kind: "header", id: 600 })).resolves.toBe(RESULT);
    expect(invoke).toHaveBeenLastCalledWith("restore_entity", { target: { kind: "header", id: 600 } });
  });

  it("a_locked_delete_rejects_with_referenced_and_the_references", async () => {
    const error = { code: "referenced", message: "Trainer 0901 wird verwendet.", asset: null, problems: [], references: [REF] };
    invoke.mockRejectedValueOnce(error);
    const caught: unknown = await deleteEntity(TRAINER, false).catch((e: unknown) => e);
    expect(isCmdError(caught)).toBe(true);
    if (!isCmdError(caught)) return;
    expect(caught.code).toBe("referenced");
    expect(caught.references).toEqual([REF]);
  });

  it("names_the_target_like_the_kernel_and_the_tombstone_of_a_diff_subject", () => {
    expect(deleteTargetLabel(TRAINER)).toBe("Trainer 0901");
    expect(deleteTargetLabel({ kind: "matrix", index: 3 })).toBe("Matrix 0003");
    expect(deleteTargetLabel({ kind: "area_data", index: 3 })).toBe("Gebietsdaten 0003");
    expect(deleteTargetLabel({ kind: "event_file", index: 17 })).toBe("Eventdatei 0017");
    expect(deleteTargetLabel({ kind: "header", id: 12 })).toBe("Header 0012");
    expect(deleteTargetLabel({ kind: "script", file: 40 })).toBe("Scriptdatei 0040");
    expect(deleteTargetLabel({ kind: "text_archive", index: 5 })).toBe("Textarchiv 0005");
    expect(deleteTargetLabel({ kind: "place", index: 120 })).toBe("Ort 120");
    expect(deleteTargetLabel({ kind: "flag", number: 0x94 })).toBe("Flag 0x0094");
    expect(deleteTargetLabel({ kind: "var", number: 0x4020 })).toBe("Variable 0x4020");
    // The entities with a tombstone, as the kernel's `tombstone_of`: a header by its id, no script, member or file.
    expect(tombstoneOf({ kind: "trainer", index: 901 })).toEqual(TRAINER);
    expect(tombstoneOf({ kind: "header", index: 600 })).toEqual({ kind: "header", id: 600 });
    expect(tombstoneOf({ kind: "event_file", index: 17 })).toEqual({ kind: "event_file", index: 17 });
    expect(tombstoneOf({ kind: "script", index: 40 })).toBeNull();
    expect(tombstoneOf({ kind: "member", folder: "maps", index: 3 })).toBeNull();
    expect(tombstoneOf({ kind: "file", path: "g4-world.json" })).toBeNull();
    // Freeing, names and places write files at once; a tombstone waits for the save.
    expect(deleteWritesFiles(TRAINER, false)).toBe(false);
    expect(deleteWritesFiles(TRAINER, true)).toBe(true);
    expect(deleteWritesFiles({ kind: "place", index: 120 }, false)).toBe(true);
    expect(deleteWritesFiles({ kind: "flag", number: 0x94 }, false)).toBe(true);
    expect(deleteWritesFiles({ kind: "var", number: 0x4020 }, false)).toBe(true);
  });
});
