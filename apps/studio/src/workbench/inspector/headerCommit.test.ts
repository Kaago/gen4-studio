import { describe, expect, it } from "vitest";
import type { MapHeader, MapHeaderPt } from "../../api";
import { HeaderSession } from "./headerCommit";
import type { CommitBackend } from "./headerCommit";

const base: MapHeaderPt = {
  area_data_id: 0,
  unknown1: 0,
  matrix_id: 0,
  script_file_id: 0,
  level_script_id: 0,
  text_archive_id: 0,
  music_day_id: 0,
  music_night_id: 0,
  wild_pokemon: 0,
  event_file_id: 0,
  location_name: 0,
  area_icon: 0,
  weather_id: 0,
  camera_angle_id: 0,
  location_specifier: 0,
  battle_background: 0,
  flags: 0,
};

/** Header A (id 1) has matrix 10, header B (id 2) has matrix 20; writes can be held back. */
function fakeBackend() {
  const table = new Map<number, MapHeader>([
    [1, { Pt: { ...base, matrix_id: 10 } }],
    [2, { Pt: { ...base, matrix_id: 20 } }],
  ]);
  const writes: { id: number; header: MapHeader; label: string }[] = [];
  let gate: Promise<void> | null = null;
  let release = () => {};
  const hold = () => {
    gate = new Promise<void>((r) => (release = r));
  };
  const backend: CommitBackend = {
    getHeader: async (id) => structuredClone(table.get(id) as MapHeader),
    setHeader: async (id, header, label) => {
      await gate;
      writes.push({ id, header, label });
      table.set(id, header);
    },
  };
  return { backend, table, writes, hold, release: () => release() };
}

function hooks() {
  const calls = { states: [] as MapHeader[], refused: 0, changed: 0, errors: [] as unknown[] };
  return {
    calls,
    hooks: {
      onState: (h: MapHeader) => calls.states.push(h),
      onRefused: () => void calls.refused++,
      onChanged: () => void calls.changed++,
      onError: (e: unknown) => calls.errors.push(e),
    },
  };
}

const matrix = (h: MapHeader) => ("Pt" in h ? h.Pt.matrix_id : -1);

describe("HeaderSession", () => {
  it("writes the edited field on top of the loaded header", async () => {
    const f = fakeBackend();
    const h = hooks();
    const a = new HeaderSession(1, f.backend, h.hooks);
    await a.load();
    await a.commit("weather_id", "Wetter", 3);
    expect(f.writes).toHaveLength(1);
    expect(f.writes[0].id).toBe(1);
    expect(f.writes[0].label).toBe("Wetter");
    expect(f.writes[0].header).toEqual({ Pt: { ...base, matrix_id: 10, weather_id: 3 } });
    expect(h.calls.changed).toBe(1);
    // Setting the same value again records nothing.
    await a.commit("weather_id", "Wetter", 3);
    expect(f.writes).toHaveLength(1);
  });

  it("scenario A: an in-flight commit of A never leaks A's data into an edit of B", async () => {
    const f = fakeBackend();
    const hA = hooks();
    const hB = hooks();
    const a = new HeaderSession(1, f.backend, hA.hooks);
    await a.load();
    f.hold();
    const pending = a.commit("matrix_id", "Matrix", 11);
    // The user selects B: A's editor is disposed and B gets its own session.
    a.dispose();
    const b = new HeaderSession(2, f.backend, hB.hooks);
    await b.load();
    f.release();
    await pending;
    await b.commit("weather_id", "Wetter", 5);
    const toB = f.writes.filter((w) => w.id === 2);
    expect(toB).toHaveLength(1);
    // B kept its own matrix and got only the weather edit.
    expect(toB[0].header).toEqual({ Pt: { ...base, matrix_id: 20, weather_id: 5 } });
    // A's write still reached A, and B's editor never saw A's values.
    expect(f.writes.find((w) => w.id === 1)?.header).toEqual({ Pt: { ...base, matrix_id: 11 } });
    expect(hB.calls.states.map(matrix)).toEqual([20, 20]);
    // The disposed A editor got no state, but the app was told to refresh.
    expect(hA.calls.states.map(matrix)).toEqual([10]);
    expect(hA.calls.changed).toBe(1);
  });

  it("scenario B: a second queued commit of A still targets A with A's base after the switch", async () => {
    const f = fakeBackend();
    const hA = hooks();
    const a = new HeaderSession(1, f.backend, hA.hooks);
    await a.load();
    f.hold();
    const first = a.commit("matrix_id", "Matrix", 11);
    const second = a.commit("weather_id", "Wetter", 2);
    a.dispose();
    const b = new HeaderSession(2, f.backend, hooks().hooks);
    await b.load();
    f.release();
    await Promise.all([first, second]);
    expect(f.writes.map((w) => w.id)).toEqual([1, 1]);
    expect(f.writes[1].header).toEqual({ Pt: { ...base, matrix_id: 11, weather_id: 2 } });
    expect(f.table.get(2)).toEqual({ Pt: { ...base, matrix_id: 20 } });
  });

  it("a reload after undo is ordered behind pending commits and replaces the base", async () => {
    const f = fakeBackend();
    const h = hooks();
    const a = new HeaderSession(1, f.backend, h.hooks);
    await a.load();
    await a.commit("matrix_id", "Matrix", 11);
    // The header was changed behind the editor's back (undo).
    f.table.set(1, { Pt: { ...base, matrix_id: 10 } });
    await a.load();
    await a.commit("weather_id", "Wetter", 4);
    expect(f.writes[1].header).toEqual({ Pt: { ...base, matrix_id: 10, weather_id: 4 } });
  });

  it("reports a refused edit, keeps the old base and reaches the app's error hook", async () => {
    const f = fakeBackend();
    const h = hooks();
    const failing: CommitBackend = {
      ...f.backend,
      setHeader: async () => {
        throw { code: "error", message: "does not fit", asset: null };
      },
    };
    const a = new HeaderSession(1, failing, h.hooks);
    await a.load();
    await a.commit("flags", "Flags", 99);
    expect(h.calls.refused).toBe(1);
    expect(h.calls.errors).toHaveLength(1);
    expect(h.calls.changed).toBe(0);
    // The failed value is not remembered: asking for it again tries again.
    await a.commit("flags", "Flags", 99);
    expect(h.calls.errors).toHaveLength(2);
  });

  it("does nothing before the header has loaded", async () => {
    const f = fakeBackend();
    const a = new HeaderSession(1, f.backend, hooks().hooks);
    await a.commit("flags", "Flags", 1);
    expect(f.writes).toHaveLength(0);
  });
});
