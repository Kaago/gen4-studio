import { describe, expect, it } from "vitest";
import { PAINT_TOOLS, paintKey, paintPatch, paintStatusHints } from "./paintTools";
import { TOOLS } from "./tools";

const BRUSH = { sub: "brush" as const, layer: 0, tile: "0041", ramp: { dir: "e" as const, steps: 1 as const }, lower: false };

describe("paintTools", () => {
  it("keys_are_the_tiled_ones", () => {
    expect(PAINT_TOOLS.map((t) => [t.id, t.key, t.label, t.icon])).toEqual([
      ["brush", "B", "Pinsel", "paint"],
      ["rect", "P", "Rechteck", "rectangle"],
      ["fill", "G", "Füllen", "bucket"],
      ["erase", "E", "Radierer", "eraser"],
      ["pick", "I", "Pipette", "eyedropper"],
      ["height", "H", "Höhe", "heights"],
      ["ramp", "R", "Rampe", "ramp"],
    ]);
    for (const t of PAINT_TOOLS) expect(paintKey(t.key.toLowerCase())).toEqual({ sub: t.id });
    // Shift is no obstacle: the upper case letter does the same.
    expect(paintKey("G")).toEqual({ sub: "fill" });
  });

  it("paint_key_maps_turn_flip_and_steps", () => {
    expect(paintKey("z")).toEqual({ turn: true });
    expect(paintKey("Z")).toEqual({ turn: true });
    expect(paintKey("x")).toEqual({ flip: true });
    expect(paintKey("1")).toEqual({ steps: 1 });
    expect(paintKey("2")).toEqual({ steps: 2 });
    expect(paintKey("3")).toBeNull();
    expect(paintKey("v")).toBeNull();
    expect(paintKey("Escape")).toBeNull();
    // Z turns the tile, or the ramp while the ramp is the sub-tool; X mirrors the tile; 1 and 2 set the steps.
    expect(paintPatch(BRUSH, { turn: true })).toEqual({ tile: "0041r1" });
    expect(paintPatch({ ...BRUSH, sub: "ramp" }, { turn: true })).toEqual({ ramp: { dir: "s", steps: 1 } });
    expect(paintPatch(BRUSH, { flip: true })).toEqual({ tile: "0041f" });
    expect(paintPatch(BRUSH, { steps: 2 })).toEqual({ ramp: { dir: "e", steps: 2 } });
    expect(paintPatch(BRUSH, { sub: "height" })).toEqual({ sub: "height" });
  });

  it("no_paint_key_clashes_with_select_figure_door_trigger_sign", () => {
    const kept = TOOLS.filter((t) => ["select", "figure", "door", "trigger", "sign"].includes(t.id)).map((t) => t.key);
    expect(kept.sort()).toEqual(["A", "F", "S", "T", "V"]);
    const ours = [...PAINT_TOOLS.map((t) => t.key), "Z", "X"];
    expect(ours.filter((k) => kept.includes(k))).toEqual([]);
    for (const k of kept) expect(paintKey(k)).toBeNull();
  });

  it("status_hints_follow_the_sub_tool", () => {
    const text = (hints: { keys: string; does: string }[]) => hints.map((h) => `${h.keys} ${h.does}`).join(" · ");
    expect(text(paintStatusHints(BRUSH))).toBe("Ziehen malt · Rechtsklick nimmt auf · Z dreht · X spiegelt");
    expect(text(paintStatusHints({ ...BRUSH, sub: "height" }))).toBe("Klick hebt · ⇧Klick senkt · Ziehen über mehrere");
    expect(text(paintStatusHints({ ...BRUSH, sub: "ramp" }))).toBe("Klick setzt Rampe nach Osten, 1 Stufe · Z dreht · 1/2 Stufen · ⇧Klick entfernt");
    expect(text(paintStatusHints({ ...BRUSH, sub: "ramp", ramp: { dir: "n", steps: 2 } }))).toContain("Klick setzt Rampe nach Norden, 2 Stufen");
  });
});
