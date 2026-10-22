import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { fetchCurrent, sceneKey } from "../reload";
import { initialSync, isCurrentSelection, sceneSync } from "./sceneSync";
import type { SyncAction, SyncOutcome, SyncState } from "./sceneSync";

const header = (id: number, matrix: number, area: number, events = id): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
  matrix_id: matrix,
  area_data_id: area,
  event_file_id: events,
  text_archive_id: 0,
});

/** Runs actions in order the way the hook does and keeps the latest state. */
function run(start: SyncState, ...actions: SyncAction[]): { state: SyncState; outcomes: SyncOutcome[] } {
  let state = start;
  const outcomes: SyncOutcome[] = [];
  for (const action of actions) {
    const result = sceneSync(state, action);
    state = result.state;
    outcomes.push(result.outcome);
  }
  return { state, outcomes };
}

describe("sceneSync: header selection", () => {
  it("remembers the scene key and area of the selected header and counts the load", () => {
    const h = header(1, 4, 9);
    const { state, outcomes } = run(initialSync(), { type: "select", header: h });
    expect(outcomes[0]).toEqual({ kind: "load", seq: 1 });
    expect(state).toMatchObject({ key: sceneKey(h), area: 9, loads: 1, selectSeq: 1 });
  });

  it("drops the result of an earlier selection when a quick second one follows", () => {
    const first = header(1, 4, 9);
    const second = header(2, 5, 10);
    const { state, outcomes } = run(
      initialSync(),
      { type: "select", header: first },
      { type: "select", header: second },
      { type: "loaded", seq: 1 },
      { type: "loaded", seq: 2 },
    );
    expect(outcomes[2]).toEqual({ kind: "stale" });
    // The newer result applies with the newer header's area data.
    expect(outcomes[3]).toEqual({ kind: "apply", area: 10 });
    expect(state.key).toBe(sceneKey(second));
  });

  it("does not let a stale failure reset the scene of the newer selection", () => {
    const { state, outcomes } = run(
      initialSync(),
      { type: "select", header: header(1, 4, 9) },
      { type: "select", header: header(2, 5, 10) },
      { type: "loadFailed", seq: 1, shownArea: 3 },
    );
    expect(outcomes[2]).toEqual({ kind: "stale" });
    expect(state.key).toBe(sceneKey(header(2, 5, 10)));
    expect(state.area).toBe(10);
  });

  it("a failed getMatrix forgets the scene key and falls back to the shown area", () => {
    const shown = header(1, 4, 9);
    const next = header(2, 5, 10);
    const { state, outcomes } = run(
      initialSync(),
      { type: "select", header: shown },
      { type: "loaded", seq: 1 },
      { type: "loadEnded" },
      { type: "select", header: next },
      { type: "loadFailed", seq: 2, shownArea: 9 },
      { type: "loadEnded" },
    );
    expect(outcomes[4]).toEqual({ kind: "reset" });
    expect(state).toMatchObject({ key: null, area: 9, loads: 0 });
    // The next sync of that header therefore rebuilds the scene instead of pairing old maps with the new area.
    const resync = run(state, { type: "listStart" }, { type: "listed", seq: 1, header: next });
    expect(resync.outcomes[1]).toEqual({ kind: "load", seq: 3 });
  });
});

describe("sceneSync: header list reads", () => {
  it("numbers the reads", () => {
    const { outcomes } = run(initialSync(), { type: "listStart" }, { type: "listStart" });
    expect(outcomes).toEqual([{ kind: "list", seq: 1 }, { kind: "list", seq: 2 }]);
  });

  it("drops a list that a newer read overtook", () => {
    const { outcomes } = run(
      initialSync(),
      { type: "listStart" },
      { type: "listStart" },
      { type: "listed", seq: 1, header: header(1, 4, 9) },
      { type: "listed", seq: 2, header: undefined },
    );
    expect(outcomes[2]).toEqual({ kind: "stale" });
    expect(outcomes[3]).toEqual({ kind: "noHeader" });
  });

  it("a changed area data without another matrix reloads only the models", () => {
    const before = header(1, 4, 9);
    const after = header(1, 4, 11);
    const { state, outcomes } = run(
      initialSync(),
      { type: "select", header: before },
      { type: "loaded", seq: 1 },
      { type: "loadEnded" },
      { type: "listStart" },
      { type: "listed", seq: 1, header: after },
    );
    expect(outcomes[4]).toEqual({ kind: "area", area: 11, apply: true });
    // The matrix was not fetched again and the scene key is untouched.
    expect(state).toMatchObject({ key: sceneKey(before), area: 11, loads: 0, selectSeq: 1 });
  });

  it("holds the area back while a scene matrix is still being fetched", () => {
    const { outcomes, state } = run(
      initialSync(),
      { type: "select", header: header(1, 4, 9) },
      { type: "listStart" },
      { type: "listed", seq: 1, header: header(1, 4, 11) },
    );
    expect(outcomes[2]).toEqual({ kind: "area", area: 11, apply: false });
    // The loaded matrix then applies together with the newest area data.
    expect(sceneSync(state, { type: "loaded", seq: 1 }).outcome).toEqual({ kind: "apply", area: 11 });
  });

  it("another matrix or event file reloads the scene under a new selection guard", () => {
    const { state, outcomes } = run(
      initialSync(),
      { type: "select", header: header(1, 4, 9) },
      { type: "loaded", seq: 1 },
      { type: "loadEnded" },
      { type: "listStart" },
      { type: "listed", seq: 1, header: header(1, 6, 9) },
      // A click on another header meanwhile wins over the sync's load.
      { type: "select", header: header(2, 7, 9) },
      { type: "loaded", seq: 2 },
    );
    expect(outcomes[4]).toEqual({ kind: "load", seq: 2 });
    expect(outcomes[6]).toEqual({ kind: "stale" });
    expect(state.selectSeq).toBe(3);
    expect(state.loads).toBe(2);
  });
});

describe("sceneSync: history revision", () => {
  it("fetches again when an undo landed while the matrix was in flight", async () => {
    let state = initialSync();
    let calls = 0;
    const next = await fetchCurrent(
      async () => {
        calls++;
        if (calls === 1) state = sceneSync(state, { type: "historyMoved" }).state;
        return calls;
      },
      () => state.history,
    );
    expect(calls).toBe(2);
    expect(next).toBe(2);
    expect(state.history).toBe(1);
  });

  it("is not touched by anything but undo and redo", () => {
    const { state } = run(initialSync(), { type: "select", header: header(1, 4, 9) }, { type: "opened" });
    expect(state.history).toBe(0);
  });
});

describe("sceneSync: tilesets", () => {
  it("only records the tilesets of a freshly loaded scene", () => {
    const { state, outcomes } = run(initialSync(), { type: "tilesetRead", area: 0, key: "a", reload: false });
    expect(outcomes[0]).toEqual({ kind: "noted" });
    expect(state.tileset).toEqual({ area: 0, key: "a" });
  });

  it("reloads the models when the tilesets of the same area data changed", () => {
    const { outcomes } = run(
      initialSync(),
      { type: "tilesetRead", area: 0, key: "a", reload: false },
      { type: "tilesetRead", area: 0, key: "b", reload: true },
      { type: "tilesetRead", area: 0, key: "b", reload: true },
    );
    expect(outcomes).toEqual([{ kind: "noted" }, { kind: "models" }, { kind: "noted" }]);
  });

  it("reloads after an unreadable area data once it reads again, and ignores a read for another area", () => {
    const { outcomes, state } = run(
      initialSync(),
      { type: "tilesetRead", area: 0, key: "a", reload: false },
      { type: "tilesetFailed", area: 0 },
      { type: "tilesetRead", area: 0, key: "a", reload: true },
      { type: "tilesetRead", area: 5, key: "z", reload: true },
    );
    expect(outcomes).toEqual([{ kind: "noted" }, { kind: "report" }, { kind: "models" }, { kind: "stale" }]);
    expect(state.tileset).toEqual({ area: 0, key: "a" });
  });

  it("a read failure for an area the selection has left is not reported", () => {
    const { outcomes, state } = run(
      initialSync(),
      { type: "select", header: header(1, 4, 9) },
      { type: "tilesetFailed", area: 0 },
    );
    expect(outcomes[1]).toEqual({ kind: "stale" });
    expect(state.tileset).toBeNull();
  });
});

describe("sceneSync: opening a project", () => {
  it("invalidates every guard and forgets what the scene showed", () => {
    const { state } = run(
      initialSync(),
      { type: "select", header: header(1, 4, 9) },
      { type: "tilesetRead", area: 9, key: "a", reload: false },
      { type: "opened" },
    );
    expect(state).toMatchObject({ key: null, area: 0, tileset: null, selectSeq: 2, listSeq: 1 });
    // A load of the old project that is still running is dropped on arrival.
    expect(sceneSync(state, { type: "loaded", seq: 1 }).outcome).toEqual({ kind: "stale" });
    expect(sceneSync(state, { type: "listed", seq: 0, header: header(1, 4, 9) }).outcome).toEqual({ kind: "stale" });
  });
});

describe("sceneSync: selection guard", () => {
  it("reports whether a selection is still the newest", () => {
    const { state } = run(initialSync(), { type: "select", header: header(1, 4, 9) });
    expect(isCurrentSelection(state, 1)).toBe(true);
    expect(isCurrentSelection(state, 0)).toBe(false);
  });
});
