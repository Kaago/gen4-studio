import { describe, expect, it } from "vitest";
import type { AreaDataDto } from "../../api";
import { AREA_FIELDS } from "./areaFields";
import { EditSession } from "./headerCommit";
import type { RecordBackend } from "./headerCommit";

const area = (map_tileset: number): AreaDataDto => ({ family: "Hgss", buildings_tileset: 1, map_tileset, dynamic_texture_type: 0, area_type: 1, light_type: 0 });

/** Area data 1 uses tileset 10, area data 2 tileset 20; writes can be held back. */
function fakeBackend() {
  const table = new Map<number, AreaDataDto>([
    [1, area(10)],
    [2, area(20)],
  ]);
  const writes: { id: number; area: AreaDataDto; label: string }[] = [];
  let gate: Promise<void> | null = null;
  let release = () => {};
  const backend: RecordBackend<AreaDataDto> = {
    get: async (id) => structuredClone(table.get(id) as AreaDataDto),
    set: async (id, record, label) => {
      await gate;
      writes.push({ id, area: record, label });
      table.set(id, record);
    },
  };
  return {
    backend,
    table,
    writes,
    hold: () => void (gate = new Promise<void>((r) => (release = r))),
    release: () => release(),
  };
}

function hooks() {
  const calls = { states: [] as AreaDataDto[], refused: 0, changed: 0, errors: [] as unknown[] };
  return {
    calls,
    hooks: {
      onState: (a: AreaDataDto) => calls.states.push(a),
      onRefused: () => void calls.refused++,
      onChanged: () => void calls.changed++,
      onError: (e: unknown) => calls.errors.push(e),
    },
  };
}

describe("EditSession over area data", () => {
  it("writes the edited field with the field label on top of the loaded record", async () => {
    const f = fakeBackend();
    const h = hooks();
    const s = new EditSession(1, f.backend, AREA_FIELDS, h.hooks);
    await s.load();
    await s.commit("map_tileset", "Karten-Tileset", 3);
    await s.commit("area_type", "Gebietstyp", 0);
    expect(f.writes.map((w) => [w.id, w.label])).toEqual([
      [1, "Karten-Tileset"],
      [1, "Gebietstyp"],
    ]);
    expect(f.writes[1].area).toEqual({ ...area(3), area_type: 0 });
    expect(h.calls.changed).toBe(2);
    // The same value again records nothing.
    await s.commit("area_type", "Gebietstyp", 0);
    expect(f.writes).toHaveLength(2);
  });

  it("an in-flight commit of area data 1 never writes its values into area data 2", async () => {
    const f = fakeBackend();
    const a = new EditSession(1, f.backend, AREA_FIELDS, hooks().hooks);
    await a.load();
    f.hold();
    const pending = a.commit("map_tileset", "Karten-Tileset", 11);
    a.dispose();
    const b = new EditSession(2, f.backend, AREA_FIELDS, hooks().hooks);
    await b.load();
    f.release();
    await pending;
    await b.commit("light_type", "Lichttyp", 5);
    expect(f.writes.filter((w) => w.id === 2).map((w) => w.area)).toEqual([{ ...area(20), light_type: 5 }]);
    expect(f.table.get(1)).toEqual(area(11));
  });

  it("a refused edit is reported and not remembered", async () => {
    const f = fakeBackend();
    const h = hooks();
    const failing: RecordBackend<AreaDataDto> = {
      ...f.backend,
      set: async () => {
        throw { code: "error", message: "family mismatch", asset: null };
      },
    };
    const s = new EditSession(1, failing, AREA_FIELDS, h.hooks);
    await s.load();
    await s.commit("map_tileset", "Karten-Tileset", 3);
    expect(h.calls.refused).toBe(1);
    expect(h.calls.errors).toHaveLength(1);
    await s.commit("map_tileset", "Karten-Tileset", 3);
    expect(h.calls.errors).toHaveLength(2);
  });
});
