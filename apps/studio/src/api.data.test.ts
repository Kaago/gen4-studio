import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import {
  createEncounterFile,
  createTrainer,
  dataEdit,
  dataGet,
  dataList,
  dataRaw,
  fieldLabels,
  getItemIcon,
  getMonIcon,
  itemTable,
  machineTable,
  nameTables,
  refs,
  setTrainerTexts,
  splitEncounterFile,
} from "./api";

describe("data commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("calls each command with the argument names of the backend", async () => {
    await dataList("species");
    expect(invoke).toHaveBeenLastCalledWith("data_list", { kind: "species", filter: null });
    await dataList("move", { query: "flü", types: [2], pocket: null, category: 0 });
    expect(invoke).toHaveBeenLastCalledWith("data_list", { kind: "move", filter: { query: "flü", types: [2], pocket: null, category: 0 } });
    await dataGet("species", 397);
    expect(invoke).toHaveBeenLastCalledWith("data_get", { kind: "species", index: 397 });
    await dataRaw("item", 17);
    expect(invoke).toHaveBeenLastCalledWith("data_raw", { kind: "item", index: 17 });
    await nameTables();
    expect(invoke).toHaveBeenLastCalledWith("name_tables");
    await itemTable();
    expect(invoke).toHaveBeenLastCalledWith("item_table");
    await machineTable();
    expect(invoke).toHaveBeenLastCalledWith("machine_table");
    await refs({ kind: "item", id: 17 });
    expect(invoke).toHaveBeenLastCalledWith("refs", { target: { kind: "item", id: 17 } });
  });

  it("sends a patch as the backend's externally tagged enum and the label only when given", async () => {
    await dataEdit("species", 397, { Personal: { field: "attack", value: 85 } });
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "species",
      index: 397,
      patch: { Personal: { field: "attack", value: 85 } },
      label: null,
    });
    await dataEdit("species", 397, { CopyMoves: { from: 396 } }, "Staravia: TM/VM und Lernset wie Staralili");
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "species",
      index: 397,
      patch: { CopyMoves: { from: 396 } },
      label: "Staravia: TM/VM und Lernset wie Staralili",
    });
    await dataEdit("encounter", 12, { Encounter: { method: "grass", slot: 2, field: "species", value: 396 } });
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "encounter",
      index: 12,
      patch: { Encounter: { method: "grass", slot: 2, field: "species", value: 396 } },
      label: null,
    });
  });

  it("calls the trainer and encounter commands with the keys the bridge accepts", async () => {
    await createTrainer(2, "Jens");
    expect(invoke).toHaveBeenLastCalledWith("create_trainer", { class: 2, name: "Jens" });
    await setTrainerTexts(928, [{ kind: 0, text: "Los!\\r" }]);
    expect(invoke).toHaveBeenLastCalledWith("set_trainer_texts", { trainer: 928, texts: [{ kind: 0, text: "Los!\\r" }] });
    await createEncounterFile();
    expect(invoke).toHaveBeenLastCalledWith("create_encounter_file", { copyOf: null });
    await createEncounterFile(12);
    expect(invoke).toHaveBeenLastCalledWith("create_encounter_file", { copyOf: 12 });
    await splitEncounterFile(3);
    expect(invoke).toHaveBeenLastCalledWith("split_encounter_file", { header: 3 });
  });

  it("sends the trainer patches with the label the editor words", async () => {
    await dataEdit("trainer", 5, { Trainer: { field: "ai", value: 7 } });
    expect(invoke).toHaveBeenLastCalledWith("data_edit", { kind: "trainer", index: 5, patch: { Trainer: { field: "ai", value: 7 } }, label: null });
    await dataEdit("trainer", 5, { TrainerFormat: { moves: true, items: null } }, "Wanderer Jens: eigene Attacken an");
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "trainer",
      index: 5,
      patch: { TrainerFormat: { moves: true, items: null } },
      label: "Wanderer Jens: eigene Attacken an",
    });
    await dataEdit("encounter", 12, { EncounterExtra: { field: "unown_table", value: 2 } }, "Route 201: Icognito-Tabelle 0 → 2");
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "encounter",
      index: 12,
      patch: { EncounterExtra: { field: "unown_table", value: 2 } },
      label: "Route 201: Icognito-Tabelle 0 → 2",
    });
  });

  it("reads icons as bytes and treats an empty body as no icon", async () => {
    invoke.mockResolvedValueOnce(new Uint8Array([0x89, 0x50]).buffer);
    const icon = await getItemIcon(17);
    expect(invoke).toHaveBeenLastCalledWith("get_item_icon", { item: 17 });
    expect(icon?.byteLength).toBe(2);
    invoke.mockResolvedValueOnce(new ArrayBuffer(0));
    expect(await getMonIcon(397)).toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("get_mon_icon", { species: 397 });
    // A transport that hands an empty body over as null is no icon either, not a TypeError.
    invoke.mockResolvedValueOnce(null);
    expect(await getItemIcon(18)).toBeNull();
  });
});

describe("fieldLabels", () => {
  it("names the fields in German without abbreviations", () => {
    expect(fieldLabels.species.attack).toBe("Angriff");
    expect(fieldLabels.species.sp_attack).toBe("Spezial-Angriff");
    expect(fieldLabels.species.speed).toBe("Initiative");
    expect(fieldLabels.species.hp).toBe("KP");
    expect(fieldLabels.species.catch_rate).toBe("Fangrate");
    expect(fieldLabels.move.power).toBe("Stärke");
    expect(fieldLabels.move.accuracy).toBe("Genauigkeit");
    expect(fieldLabels.move.pp).toBe("AP");
    expect(fieldLabels.move.priority).toBe("Priorität");
    expect(fieldLabels.item.price).toBe("Preis");
    expect(fieldLabels.encounter.grass).toBe("Gras");
    expect(fieldLabels.encounter.radio_sinnoh).toBe("Radio Sinnoh");
    expect(fieldLabels.trainer.battle_type).toBe("Kampfart");
    expect(fieldLabels.trainer.item0).toBe("Gegenstand 1");
    for (const table of Object.values(fieldLabels)) {
      for (const label of Object.values(table)) expect(label).not.toMatch(/\./);
    }
  });
});
