import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import {
  createEncounterFile,
  createTrainer,
  dataEdit,
  dataGet,
  dataList,
  dataRaw,
  fieldLabel,
  fieldLabels,
  getBattleSprite,
  getItemIcon,
  getMonIcon,
  getTrainerSprite,
  itemTable,
  machineTable,
  nameTables,
  refs,
  setTrainerTexts,
  splitEncounterFile,
} from "./api";
import type { DataPatch, NameTablesDto, SoundNameDto } from "./api";

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
    await dataEdit("species", 1, { EggMoves: { moves: [73, 75] } });
    expect(invoke).toHaveBeenLastCalledWith("data_edit", { kind: "species", index: 1, patch: { EggMoves: { moves: [73, 75] } }, label: null });
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

  it("sends a slot swap as one patch and reads the sound names", async () => {
    const swap: DataPatch = { EncounterSwap: { method: "grass", a: 2, b: 7 } };
    await dataEdit("encounter", 12, swap);
    expect(invoke).toHaveBeenLastCalledWith("data_edit", {
      kind: "encounter",
      index: 12,
      patch: { EncounterSwap: { method: "grass", a: 2, b: 7 } },
      label: null,
    });
    const sounds: SoundNameDto[] = [
      { id: 1000, name: "SEQ_DUMMY", used_in: "Beta version of Rt. 206 (Plays in mystery zone)" },
      { id: 1001, name: "SEQ_TOWN01", used_in: null },
    ];
    invoke.mockResolvedValueOnce({ sounds } satisfies Partial<NameTablesDto>);
    const tables = await nameTables();
    expect(tables.sounds.map((s) => [s.id, s.name])).toEqual([
      [1000, "SEQ_DUMMY"],
      [1001, "SEQ_TOWN01"],
    ]);
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

  it("reads battle and trainer sprites with the argument names of the backend", async () => {
    invoke.mockResolvedValueOnce(new Uint8Array([0x89, 0x50]).buffer);
    const sprite = await getBattleSprite({ species: 25, side: "back", female: true, shiny: true, frame: 1 });
    expect(sprite?.byteLength).toBe(2);
    expect(invoke).toHaveBeenLastCalledWith("get_battle_sprite", { species: 25, side: "back", female: true, shiny: true, frame: 1 });
    expect(Object.keys(invoke.mock.lastCall?.[1] ?? {}).sort()).toEqual(["female", "form", "frame", "shiny", "side", "species"]);
    invoke.mockResolvedValueOnce(new ArrayBuffer(0));
    expect(await getBattleSprite({ species: 600, form: 0, side: "front" })).toBeNull();
    invoke.mockResolvedValueOnce(new Uint8Array([0x89]).buffer);
    expect((await getTrainerSprite(3))?.byteLength).toBe(1);
    expect(invoke).toHaveBeenLastCalledWith("get_trainer_sprite", { class: 3 });
    invoke.mockResolvedValueOnce(null);
    expect(await getTrainerSprite(200)).toBeNull();
  });
});

describe("fieldLabels", () => {
  it("names the fields in German without abbreviations", () => {
    expect(fieldLabels.species.attack).toBe("Angriff");
    expect(fieldLabels.species.sp_attack).toBe("Spezial-Angriff");
    expect(fieldLabels.species.speed).toBe("Initiative");
    expect(fieldLabels.species.hp).toBe("KP");
    expect(fieldLabels.species.catch_rate).toBe("Fangrate");
    expect(fieldLabels.species.egg_moves).toBe("Ei-Attacken");
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

describe("fieldLabel", () => {
  afterEach(() => i18next.changeLanguage("de"));

  it("reads every field of fieldLabels from the catalog, in German the table's own word", () => {
    for (const [kind, table] of Object.entries(fieldLabels)) {
      for (const [field, german] of Object.entries(table)) expect(fieldLabel(kind as keyof typeof fieldLabels, field as never), `${kind}.${field}`).toBe(german);
    }
  });

  it("names a field in English when English is shown", async () => {
    await i18next.changeLanguage("en");
    expect(fieldLabel("species", "sp_attack")).toBe("Sp. Atk");
    expect(fieldLabel("move", "power")).toBe("Power");
    for (const [kind, table] of Object.entries(fieldLabels)) {
      for (const field of Object.keys(table)) expect(i18next.exists(`common:fieldLabels.${kind}.${field}`, { lng: "en", fallbackLng: false }), `${kind}.${field}`).toBe(true);
    }
  });
});
