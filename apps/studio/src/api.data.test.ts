import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import {
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

  it("reads icons as bytes and treats an empty body as no icon", async () => {
    invoke.mockResolvedValueOnce(new Uint8Array([0x89, 0x50]).buffer);
    const icon = await getItemIcon(17);
    expect(invoke).toHaveBeenLastCalledWith("get_item_icon", { item: 17 });
    expect(icon?.byteLength).toBe(2);
    invoke.mockResolvedValueOnce(new ArrayBuffer(0));
    expect(await getMonIcon(397)).toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("get_mon_icon", { species: 397 });
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
    for (const table of Object.values(fieldLabels)) {
      for (const label of Object.values(table)) expect(label).not.toMatch(/\./);
    }
  });
});
