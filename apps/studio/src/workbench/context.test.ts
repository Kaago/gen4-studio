import { describe, expect, it } from "vitest";
import type { EventsDto } from "../api";
import { contextCounts, groupEvents } from "./context";

const events = (over: Partial<EventsDto> = {}): EventsDto => ({
  event_file_id: 7,
  locked: null,
  bg: [{ index: 0, script: 12, event_type: 0, x: 4, z: 5, y: 0 }],
  objects: [
    { index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, trainer_type: 0, sight: 0, x: 10, z: 20, y: 0 },
    { index: 1, local_id: 2, graphics_id: 4, movement_type: 0, script: 3, dir: 0, trainer_type: 0, sight: 0, x: 11, z: 21, y: 0 },
  ],
  warps: [{ index: 0, x: 8, z: 9, dest_header_id: 414, dest_warp_id: 0, height: 0 }],
  coords: [],
  ...over,
});

describe("groupEvents", () => {
  it("groups_by_class_in_the_palette_order_and_leaves_empty_classes_out", () => {
    const groups = groupEvents(events());
    expect(groups.map((g) => [g.kind, g.label, g.rows.length])).toEqual([
      ["object", "Figuren", 2],
      ["warp", "Türen", 1],
      ["bg", "Schilder", 1],
    ]);
  });

  it("titles_each_row_like_the_inspector_heading_and_puts_the_position_in_the_technical_part", () => {
    const [figures, doors, signs] = groupEvents(events());
    expect(figures.rows[1]).toEqual({ index: 1, title: "Objekt 1 · ID 2", tech: "x 11, z 21" });
    expect(doors.rows[0]).toEqual({ index: 0, title: "Warp 0 → Header 0414", tech: "x 8, z 9", dest: 414 });
    expect(signs.rows[0]?.title).toBe("Hintergrund-Event 0 · Script 12");
  });

  it("names_the_header_a_door_leads_to_when_asked", () => {
    const doors = groupEvents(events(), (h) => (h === 414 ? "T01R0201" : undefined)).find((g) => g.kind === "warp");
    expect(doors?.rows[0]?.title).toBe("Warp 0 → Header 0414 (T01R0201)");
  });

  it("titles_a_trainer_figure_by_class_and_name_when_asked", () => {
    const trainer = { ...events().objects[0]!, script: 3355 };
    const figures = groupEvents(events({ objects: [trainer, events().objects[1]!] }), undefined, (script) => (script === 3355 ? "Teenager Sebastian" : null))[0]!;
    expect(figures.rows[0]).toEqual({ index: 0, title: "Objekt 0 · Teenager Sebastian", tech: "x 10, z 20" });
    // A figure that starts no battle keeps its title.
    expect(figures.rows[1]?.title).toBe("Objekt 1 · ID 2");
  });

  it("is_empty_for_a_file_without_events", () => {
    expect(groupEvents(events({ bg: [], objects: [], warps: [], coords: [] }))).toEqual([]);
  });
});

describe("contextCounts", () => {
  it("counts_each_class_with_its_singular_or_plural", () => {
    expect(contextCounts(events())).toBe("2 Figuren, 1 Tür, 1 Schild");
    expect(contextCounts(events({ objects: [events().objects[0]!], warps: [...events().warps, { ...events().warps[0]!, index: 1 }], coords: [{ index: 0, script: 1, x: 1, z: 1, width: 1, length: 1 }], bg: [] }))).toBe(
      "1 Figur, 2 Türen, 1 Auslöser",
    );
  });

  it("appends_the_buildings_and_says_nothing_for_an_empty_place", () => {
    expect(contextCounts(events({ bg: [], objects: [], warps: [] }), 3)).toBe("3 Gebäude");
    expect(contextCounts(events({ bg: [], objects: [], warps: [] }), 1)).toBe("1 Gebäude");
    expect(contextCounts(events({ bg: [], objects: [], warps: [] }))).toBe("Noch nichts");
    expect(contextCounts(null)).toBe("Noch nichts");
  });
});
