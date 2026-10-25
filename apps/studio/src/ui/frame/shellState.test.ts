// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { HAND_DEFAULT, HAND_MAX, HAND_MIN, readShellState, rememberRecent, writeShellState } from "./shellState";
import type { RecentProject } from "./shellState";

const KEY = "g4.shell.v1";
const project = (n: number, openedAt = n): RecentProject => ({ path: `/p/${n}`, name: `p${n}`, game: "Platin, DE", openedAt });

describe("shell state", () => {
  beforeEach(() => localStorage.clear());

  it("has_the_defaults_when_nothing_is_stored", () => {
    const s = readShellState();
    expect(s.area).toBe("start");
    expect(s.depth).toBe("bauen");
    expect(s.recent).toEqual([]);
    expect(s.listOpen.welt).toBe(true);
    expect(s.handHeight.welt).toBe(286);
    expect(s.handCollapsed.welt).toBe(false);
    expect([HAND_MIN, HAND_MAX, HAND_DEFAULT]).toEqual([200, 560, 286]);
  });

  it("round_trips_what_it_wrote", () => {
    const s = readShellState();
    const next = { ...s, area: "welt" as const, depth: "tuefteln" as const, listOpen: { ...s.listOpen, welt: false }, handHeight: { ...s.handHeight, welt: 320 }, handCollapsed: { ...s.handCollapsed, welt: true }, recent: [project(1)] };
    writeShellState(next);
    expect(readShellState()).toEqual(next);
  });

  it("clamps_a_stored_hand_height_to_200_to_560", () => {
    const s = readShellState();
    localStorage.setItem(KEY, JSON.stringify({ ...s, handHeight: { welt: 1000, texte: 10, technik: 0, start: 2000, geschichten: 300 } }));
    const read = readShellState();
    expect(read.handHeight.welt).toBe(560);
    expect(read.handHeight.texte).toBe(200);
    expect(read.handHeight.technik).toBe(200);
    expect(read.handHeight.start).toBe(560);
    expect(read.handHeight.geschichten).toBe(300);
  });

  it("gives_a_default_to_a_height_that_is_not_a_number", () => {
    localStorage.setItem(KEY, JSON.stringify({ handHeight: { welt: "tall", texte: null, technik: Number.NaN } }));
    const read = readShellState();
    expect(read.handHeight.welt).toBe(HAND_DEFAULT);
    expect(read.handHeight.texte).toBe(HAND_DEFAULT);
    expect(read.handHeight.technik).toBe(HAND_DEFAULT);
  });

  it("falls_back_to_the_defaults_on_broken_json_or_a_wrong_shape", () => {
    const defaults = readShellState();
    localStorage.setItem(KEY, "{not json");
    expect(readShellState()).toEqual(defaults);
    localStorage.setItem(KEY, "42");
    expect(readShellState()).toEqual(defaults);
    localStorage.setItem(KEY, "null");
    expect(readShellState()).toEqual(defaults);
  });

  it("replaces_invalid_fields_one_by_one_and_keeps_the_valid_ones", () => {
    localStorage.setItem(KEY, JSON.stringify({ area: "nowhere", depth: "tuefteln", listOpen: { welt: "yes", texte: false }, handCollapsed: { welt: true, texte: 3 }, recent: "none" }));
    const read = readShellState();
    expect(read.area).toBe("start");
    expect(read.depth).toBe("tuefteln");
    expect(read.listOpen.welt).toBe(true);
    expect(read.listOpen.texte).toBe(false);
    expect(read.handCollapsed.welt).toBe(true);
    expect(read.handCollapsed.texte).toBe(false);
    expect(read.recent).toEqual([]);
  });

  it("reads_v1_without_the_data_fields_with_their_defaults", () => {
    // A state written before the data areas: no `sub`, no `selected`.
    localStorage.setItem(KEY, JSON.stringify({ area: "texte", depth: "bauen", recent: [] }));
    const read = readShellState();
    expect(read.area).toBe("texte");
    expect(read.sub).toEqual({ pokemon: "pokedex" });
    expect(read.selected).toEqual({});
  });

  it("reads_v1_with_the_sub_area_and_the_selection_per_sub_area", () => {
    localStorage.setItem(KEY, JSON.stringify({ area: "pokemon", sub: { pokemon: "moves" }, selected: { pokedex: 397, moves: 17, items: 4 } }));
    const read = readShellState();
    expect(read.area).toBe("pokemon");
    expect(read.sub.pokemon).toBe("moves");
    expect(read.selected).toEqual({ pokedex: 397, moves: 17, items: 4 });
  });

  it("drops_an_unknown_sub_area_and_selections_that_are_no_index", () => {
    localStorage.setItem(KEY, JSON.stringify({ sub: { pokemon: "shops" }, selected: { pokedex: -1, moves: 1.5, items: "4", other: 3 } }));
    const read = readShellState();
    expect(read.sub.pokemon).toBe("pokedex");
    expect(read.selected).toEqual({});
  });

  it("reads_and_writes_world_sort_in_the_shell_state", () => {
    expect(readShellState().worldSort).toEqual({ tree: "game", list: "id" });
    localStorage.setItem(KEY, JSON.stringify({ worldSort: { tree: "recent", list: "code" } }));
    expect(readShellState().worldSort).toEqual({ tree: "recent", list: "code" });
    localStorage.setItem(KEY, JSON.stringify({ worldSort: { tree: "name" } }));
    expect(readShellState().worldSort).toEqual({ tree: "name", list: "id" });
    localStorage.setItem(KEY, JSON.stringify({ worldSort: { tree: "alphabet", list: 3 } }));
    expect(readShellState().worldSort).toEqual({ tree: "game", list: "id" });
    localStorage.setItem(KEY, JSON.stringify({ worldSort: "name" }));
    expect(readShellState().worldSort).toEqual({ tree: "game", list: "id" });
    writeShellState({ ...readShellState(), worldSort: { tree: "name", list: "name" } });
    expect(readShellState().worldSort).toEqual({ tree: "name", list: "name" });
  });

  it("reads_the_toolbar_anchor_and_falls_back_to_left", () => {
    expect(readShellState().toolbar).toBe("left");
    for (const anchor of ["left", "top", "bottom"] as const) {
      localStorage.setItem(KEY, JSON.stringify({ toolbar: anchor }));
      expect(readShellState().toolbar).toBe(anchor);
    }
    localStorage.setItem(KEY, JSON.stringify({ toolbar: "right" }));
    expect(readShellState().toolbar).toBe("left");
    localStorage.setItem(KEY, JSON.stringify({ toolbar: 2 }));
    expect(readShellState().toolbar).toBe("left");
  });

  it("drops_recent_entries_that_are_not_projects", () => {
    localStorage.setItem(KEY, JSON.stringify({ recent: [project(1), { path: 3 }, null, "x", { path: "/q", name: "q", game: "g" }, project(2)] }));
    expect(readShellState().recent.map((p) => p.path)).toEqual(["/p/1", "/p/2"]);
  });

  it("survives_a_storage_that_throws", () => {
    const get = Storage.prototype.getItem;
    const set = Storage.prototype.setItem;
    Storage.prototype.getItem = () => {
      throw new Error("blocked");
    };
    Storage.prototype.setItem = () => {
      throw new Error("blocked");
    };
    try {
      expect(readShellState().area).toBe("start");
      expect(() => writeShellState(readShellState())).not.toThrow();
    } finally {
      Storage.prototype.getItem = get;
      Storage.prototype.setItem = set;
    }
  });
});

describe("rememberRecent", () => {
  it("puts_the_project_first_and_removes_the_duplicate_by_path", () => {
    const list = [project(1), project(2), project(3)];
    const next = rememberRecent(list, { ...project(3), openedAt: 99 });
    expect(next.map((p) => p.path)).toEqual(["/p/3", "/p/1", "/p/2"]);
    expect(next[0]?.openedAt).toBe(99);
  });

  it("keeps_at_most_five_dropping_the_oldest", () => {
    let list: RecentProject[] = [];
    for (let n = 1; n <= 7; n++) list = rememberRecent(list, project(n));
    expect(list.map((p) => p.path)).toEqual(["/p/7", "/p/6", "/p/5", "/p/4", "/p/3"]);
  });

  it("does_not_change_the_list_it_was_given", () => {
    const list = [project(1)];
    rememberRecent(list, project(2));
    expect(list).toEqual([project(1)]);
  });
});
