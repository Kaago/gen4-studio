import { beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, itemTable, nameTables } from "../api";
import { DataStore } from "./dataStore";
import { dataChangedOf, editEventsOf } from "./dataBus";
import { ITEMS, ITEM_TABLE, MOVES, NAMES, SPECIES, projectDp, projectPt } from "./dataTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
}));

const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe("DataStore", () => {
  beforeEach(() => {
    vi.mocked(dataList).mockReset().mockImplementation(async (kind) => (kind === "species" ? SPECIES : kind === "move" ? MOVES : ITEMS));
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    vi.mocked(itemTable).mockReset().mockResolvedValue(ITEM_TABLE);
  });

  it("reads_nothing_until_a_list_is_needed_and_each_part_once", async () => {
    const store = new DataStore();
    expect(store.get().rows).toEqual({});
    store.need(projectPt, "species");
    store.need(projectPt, "species");
    await settle();
    expect(dataList).toHaveBeenCalledTimes(1);
    expect(nameTables).toHaveBeenCalledTimes(1);
    expect(itemTable).not.toHaveBeenCalled();
    expect(store.get().rows.species).toBe(SPECIES);
    expect(store.get().names).toBe(NAMES);
    store.need(projectPt, "item");
    await settle();
    expect(store.get().itemTable).toBe(ITEM_TABLE);
    expect(nameTables).toHaveBeenCalledTimes(1);
  });

  it("tells_its_readers_and_reads_a_changed_list_again_but_only_a_list_it_has", async () => {
    const store = new DataStore();
    const heard = vi.fn();
    store.subscribe(heard);
    store.need(projectPt, "species");
    await settle();
    expect(heard).toHaveBeenCalled();
    store.onEvent({ topic: "dataChanged", kind: "move", index: 3 });
    await settle();
    expect(dataList).toHaveBeenCalledTimes(1);
    store.onEvent({ topic: "dataChanged", kind: "species", index: 397 });
    await settle();
    expect(dataList).toHaveBeenCalledTimes(2);
    // A learnset is no part of the species rows.
    store.onEvent({ topic: "dataChanged", kind: "learnset", index: 397 });
    await settle();
    expect(dataList).toHaveBeenCalledTimes(2);
    // Names come from the text archives (and the rows carry them).
    store.onEvent({ topic: "textData", archive: 412 });
    await settle();
    expect(dataList).toHaveBeenCalledTimes(3);
    expect(nameTables).toHaveBeenCalledTimes(2);
  });

  it("reads_the_names_and_the_lists_it_has_again_when_a_text_archive_changed", async () => {
    const store = new DataStore();
    store.need(projectPt, "species");
    await settle();
    // A name edited in the texts: the rows carry the names, so they are read again too.
    vi.mocked(dataList).mockResolvedValue([{ ...SPECIES[2]!, name: "Staravia neu" }]);
    store.onEvent({ topic: "textData", archive: 412 });
    await settle();
    expect(store.get().rows.species?.[0]?.name).toBe("Staravia neu");
    // A list never asked for is not read.
    expect(vi.mocked(dataList).mock.calls.map((c) => c[0])).toEqual(["species", "species"]);
  });

  it("reads_again_only_what_the_changed_text_archive_names", async () => {
    const store = new DataStore();
    store.needAll(projectPt);
    await settle();
    // What was read again after `event`: the parts by name.
    const readsAfter = async (archive: number | null) => {
      vi.mocked(dataList).mockClear();
      vi.mocked(nameTables).mockClear();
      store.onEvent({ topic: "textData", archive });
      await settle();
      const parts = vi.mocked(dataList).mock.calls.map((c) => c[0] as string);
      if (vi.mocked(nameTables).mock.calls.length > 0) parts.push("names");
      return parts.sort();
    };
    // A move description, a script's texts: no name in them.
    expect(await readsAfter(646)).toEqual([]);
    expect(await readsAfter(100)).toEqual([]);
    // The move names: the names and the move rows, not the species or the items.
    expect(await readsAfter(647)).toEqual(["move", "names"]);
    expect(await readsAfter(412)).toEqual(["names", "species"]);
    expect(await readsAfter(392)).toEqual(["item", "names"]);
    // The abilities: only the names.
    expect(await readsAfter(610)).toEqual(["names"]);
    // The trainer names (Platinum 618) are no name table, the trainer rows carry them; the class names (619) are both.
    expect(await readsAfter(618)).toEqual(["trainer"]);
    expect(await readsAfter(619)).toEqual(["names", "trainer"]);
    // Any archive (a reload after a stale file): everything asked for.
    expect(await readsAfter(null)).toEqual(["item", "move", "names", "species", "trainer"]);
  });

  it("reads_the_trainer_rows_again_when_events_or_headers_changed_and_any_text_in_a_game_of_unknown_banks", async () => {
    const store = new DataStore();
    store.need(projectPt, "trainer");
    await settle();
    const trainerReads = () => vi.mocked(dataList).mock.calls.filter((c) => c[0] === "trainer").length;
    expect(trainerReads()).toBe(1);
    store.onEvent({ topic: "eventData" });
    await settle();
    store.onEvent({ topic: "headerTable" });
    await settle();
    store.onEvent({ topic: "dataChanged", kind: "trainer", index: null });
    await settle();
    expect(trainerReads()).toBe(4);
    store.onEvent({ topic: "textData", archive: 100 });
    await settle();
    expect(trainerReads()).toBe(4);
    // A Japanese Platinum: the trainer name bank is not known, any text archive may be it.
    const ja = new DataStore();
    ja.need({ ...projectPt, language: "ja" }, "trainer");
    await settle();
    vi.mocked(dataList).mockClear();
    ja.onEvent({ topic: "textData", archive: 100 });
    await settle();
    expect(vi.mocked(dataList).mock.calls.map((c) => c[0])).toEqual(["trainer"]);
  });

  it("reads_a_list_again_that_changed_while_it_was_being_read", async () => {
    const store = new DataStore();
    store.need(projectPt, "move");
    store.onEvent({ topic: "dataChanged", kind: "move", index: 17 });
    await settle();
    await settle();
    expect(vi.mocked(dataList).mock.calls.filter((c) => c[0] === "move")).toHaveLength(2);
  });

  it("starts_over_for_another_project_and_drops_late_answers_of_the_old_one", async () => {
    let answer: (rows: typeof SPECIES) => void = () => {};
    vi.mocked(dataList).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
    const store = new DataStore();
    store.need(projectPt, "species");
    store.need({ ...projectPt }, "move");
    answer(SPECIES);
    await settle();
    expect(store.get().rows.species).toBeUndefined();
    expect(store.get().rows.move).toBe(MOVES);
  });

  it("takes_the_read_only_answer_of_diamond_and_pearl_as_the_lock", async () => {
    const locked = { code: "read_only", message: "Diamant/Perl: ohne Testprojekt nicht freigegeben", asset: null };
    vi.mocked(dataList).mockRejectedValue(locked);
    vi.mocked(nameTables).mockRejectedValue(locked);
    const store = new DataStore();
    store.need(projectDp, "species");
    await settle();
    expect(store.get().locked).toBe("Diamant/Perl: ohne Testprojekt nicht freigegeben");
    expect(store.get().errors).toEqual({});
  });

  it("keeps_other_errors_per_part", async () => {
    vi.mocked(dataList).mockRejectedValue({ code: "error", message: "kaputt", asset: null });
    const store = new DataStore();
    store.need(projectPt, "move");
    await settle();
    expect(store.get().errors.move).toBe("kaputt");
    expect(store.get().locked).toBeNull();
  });
});

describe("dataChangedOf", () => {
  it("names_the_data_kind_of_a_restored_asset", () => {
    expect(dataChangedOf({ kind: "Personal", index: 397 })).toEqual({ topic: "dataChanged", kind: "species", index: 397 });
    expect(dataChangedOf({ kind: "Move", index: 17 })).toEqual({ topic: "dataChanged", kind: "move", index: 17 });
    expect(dataChangedOf({ kind: "Item", index: 17 })).toEqual({ topic: "dataChanged", kind: "item", index: 17 });
    expect(dataChangedOf({ kind: "Learnset", index: 3 })).toEqual({ topic: "dataChanged", kind: "learnset", index: 3 });
    expect(dataChangedOf({ kind: "TrainerParty", index: 3 })).toEqual({ topic: "dataChanged", kind: "trainer", index: 3 });
    expect(dataChangedOf({ kind: "Trainer", index: 3 })).toEqual({ topic: "dataChanged", kind: "trainer", index: 3 });
    // The text table and offsets hold every trainer's texts: any trainer may have changed.
    expect(dataChangedOf({ kind: "TrainerTextTable", index: 0 })).toEqual({ topic: "dataChanged", kind: "trainer", index: null });
    expect(dataChangedOf({ kind: "Map", index: 3 })).toBeNull();
  });

  it("announces a text archive a data edit changed as textData, a data member as dataChanged", () => {
    expect(editEventsOf({ kind: "TextArchive", index: 618 })).toEqual([{ topic: "textData", archive: 618 }]);
    expect(editEventsOf({ kind: "Trainer", index: 5 })).toEqual([{ topic: "dataChanged", kind: "trainer", index: 5 }]);
    expect(editEventsOf({ kind: "Map", index: 3 })).toEqual([]);
  });
});
