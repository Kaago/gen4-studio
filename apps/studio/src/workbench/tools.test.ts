import { describe, expect, it } from "vitest";
import { EVENT_TOOL_KIND, TOOLS, isEventTool, toolForKey } from "./tools";

describe("tools", () => {
  it("lists_the_seven_tools_in_palette_order_with_their_letters", () => {
    expect(TOOLS.map((t) => [t.id, t.key, t.label])).toEqual([
      ["select", "V", "Auswählen"],
      ["paint", "B", "Gelände malen"],
      ["figure", "F", "Figur"],
      ["door", "T", "Tür"],
      ["trigger", "A", "Auslöser"],
      ["sign", "S", "Schild"],
      ["building", "G", "Gebäude"],
    ]);
    expect(new Set(TOOLS.map((t) => t.key)).size).toBe(TOOLS.length);
    for (const tool of TOOLS) {
      expect(tool.hint).not.toBe("");
      expect(tool.statusHints.length).toBeGreaterThan(0);
    }
  });

  it("select_names_markers_buildings_and_maps_and_keeps_its_status_hint", () => {
    const select = TOOLS[0];
    expect(select.hint).toBe("Wählt Marker, Gebäude und Karten in der Szene. Ziehen verschiebt das Gewählte. Malt nie.");
    expect(select.statusHints[0]).toEqual({ keys: "Klick", does: "wählt" });
  });

  it("maps_a_letter_to_its_tool_in_either_case_and_any_other_key_to_null", () => {
    expect(toolForKey("f")).toBe("figure");
    expect(toolForKey("F")).toBe("figure");
    expect(toolForKey("v")).toBe("select");
    expect(toolForKey("b")).toBe("paint");
    expect(toolForKey("g")).toBe("building");
    expect(toolForKey("x")).toBeNull();
    expect(toolForKey("Escape")).toBeNull();
    expect(toolForKey("")).toBeNull();
  });

  it("maps_the_four_event_tools_to_their_event_class", () => {
    expect(EVENT_TOOL_KIND).toEqual({ figure: "object", door: "warp", trigger: "coord", sign: "bg" });
    expect(TOOLS.filter((t) => isEventTool(t.id)).map((t) => t.id)).toEqual(["figure", "door", "trigger", "sign"]);
  });
});
