import { describe, expect, it } from "vitest";
import type { EntityDiff } from "../api";
import { navTargetOf, sideWords, stateBadge } from "./diffWords";

const fields = (...paths: string[]): EntityDiff => ({ shape: "fields", changes: paths.map((path) => ({ path, old: "1", new: "2" })), more: 0 });

describe("diffWords", () => {
  it("side_words_per_against", () => {
    expect(sideWords("base")).toEqual({ old: "Basis", new: "Projekt" });
    expect(sideWords("head")).toEqual({ old: "Commit", new: "Projekt" });
    expect(sideWords("saved")).toEqual({ old: "Gespeichert", new: "Ungespeichert" });
  });

  it("badges_per_against", () => {
    expect(stateBadge("base", "changed")).toEqual({ kind: "warning", text: "Geändert" });
    expect(stateBadge("base", "new")).toEqual({ kind: "info", text: "Neu" });
    expect(stateBadge("base", "missing")).toEqual({ kind: "error", text: "Fehlt" });
    expect(stateBadge("head", "changed")).toEqual({ kind: "warning", text: "Geändert" });
    expect(stateBadge("head", "new")).toEqual({ kind: "info", text: "Neu" });
    expect(stateBadge("head", "missing")).toEqual({ kind: "error", text: "Gelöscht" });
    // Every unsaved row is just that.
    expect(stateBadge("saved", "changed")).toEqual({ kind: "unsaved", text: "Ungespeichert" });
    expect(stateBadge("saved", "new")).toEqual({ kind: "unsaved", text: "Ungespeichert" });
  });

  it("nav_target_of_each_subject_kind_and_none_for_maps_and_files", () => {
    expect(navTargetOf({ kind: "header", index: 3 }, null)).toEqual({ kind: "header", id: 3 });
    expect(navTargetOf({ kind: "area_data", index: 7 }, null)).toEqual({ kind: "area", id: 7 });
    expect(navTargetOf({ kind: "species", index: 25 }, null)).toEqual({ kind: "species", index: 25 });
    expect(navTargetOf({ kind: "move", index: 33 }, null)).toEqual({ kind: "move", index: 33 });
    expect(navTargetOf({ kind: "item", index: 4 }, null)).toEqual({ kind: "item", id: 4 });
    expect(navTargetOf({ kind: "trainer", index: 1 }, null)).toEqual({ kind: "trainer", index: 1 });
    expect(navTargetOf({ kind: "encounter", index: 12 }, null)).toEqual({ kind: "encounter", index: 12 });
    expect(navTargetOf({ kind: "matrix", index: 0 }, null)).toEqual({ kind: "matrixCell", matrix: 0, x: 0, y: 0 });
    // A text archive at its first changed message; without a diff at its first message.
    const messages: EntityDiff = {
      shape: "messages",
      messages: [
        { index: 4, state: "changed", old: [], new: [] },
        { index: 9, state: "new", old: [], new: [] },
      ],
      more: 0,
    };
    expect(navTargetOf({ kind: "text_archive", index: 433 }, messages)).toEqual({ kind: "textArchive", index: 433, message: 4 });
    expect(navTargetOf({ kind: "text_archive", index: 433 }, null)).toEqual({ kind: "textArchive", index: 433 });
    // A script at its first changed line on the new side (the context before it counts).
    const lines: EntityDiff = {
      shape: "lines",
      hunks: [
        {
          old_start: 10,
          new_start: 12,
          lines: [
            ["same", "a"],
            ["same", "b"],
            ["removed", "c"],
            ["added", "d"],
          ],
        },
      ],
      more: 0,
    };
    expect(navTargetOf({ kind: "script", index: 2 }, lines)).toEqual({ kind: "script", file: 2, line: 14 });
    expect(navTargetOf({ kind: "script", index: 2 }, null)).toEqual({ kind: "script", file: 2 });
    // An event file only when its first changed path lies in an event list: the list's key names the kind.
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("warp_events[2].x", "bg_events[0].y"))).toEqual({ kind: "event", eventFile: 5, eventKind: "warp", index: 2 });
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("object_events[1].script"))).toEqual({ kind: "event", eventFile: 5, eventKind: "object", index: 1 });
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("coord_events[0].var"))).toEqual({ kind: "event", eventFile: 5, eventKind: "coord", index: 0 });
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("bg_events[3].kind"))).toEqual({ kind: "event", eventFile: 5, eventKind: "bg", index: 3 });
    // A whole list added or a field outside the lists: no event to open.
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("warp_events"))).toBeNull();
    expect(navTargetOf({ kind: "event_file", index: 5 }, fields("id"))).toBeNull();
    expect(navTargetOf({ kind: "event_file", index: 5 }, null)).toBeNull();
    // No place in the app.
    expect(navTargetOf({ kind: "map", index: 3 }, null)).toBeNull();
    expect(navTargetOf({ kind: "file", path: "g4-world.json" }, null)).toBeNull();
    expect(navTargetOf({ kind: "member", folder: "pokeHeight", index: 5 }, null)).toBeNull();
    expect(navTargetOf({ kind: "map_texture", index: 1 }, null)).toBeNull();
    expect(navTargetOf({ kind: "building_model", index: 1 }, null)).toBeNull();
    // A story opens in Geschichten › Geschichten by its number.
    expect(navTargetOf({ kind: "story", index: 0 }, fields("notes"))).toEqual({ kind: "story", number: 0 });
    expect(navTargetOf({ kind: "story", index: 3 }, null)).toEqual({ kind: "story", number: 3 });
  });
});
