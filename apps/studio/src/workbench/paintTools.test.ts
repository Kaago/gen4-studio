import { describe, expect, it } from "vitest";
import { PAINT_TOOLS, paintKey, paintPatch, paintStatusHints } from "./paintTools";
import { TOOLS } from "./tools";

const BRUSH = { sub: "brush" as const, layer: 0, tile: "0041", ramp: { dir: "e" as const, steps: 1 as const }, lower: false, edgeSet: null, cliffRise: 2 };

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
      ["terrain", "T", "Terrain-Pinsel", "terrain"],
      ["cliff", "K", "Klippenpinsel", "cliff"],
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

  it("no_paint_key_clashes", () => {
    const kept = TOOLS.filter((t) => ["select", "figure", "door", "trigger", "sign"].includes(t.id)).map((t) => t.key);
    expect(kept.sort()).toEqual(["A", "F", "S", "T", "V"]);
    const ours = [...PAINT_TOOLS.map((t) => t.key), "Z", "X"];
    expect(ours.sort()).toEqual(["B", "E", "G", "H", "I", "K", "P", "R", "T", "X", "Z"]);
    // B P G E I H R T K Z X ∩ {V, F, A, S} is empty; T beats „Tür“ only while painting (the shortcuts ask `paintKey` first then).
    expect(ours.filter((k) => ["V", "F", "A", "S"].includes(k))).toEqual([]);
    for (const k of ["V", "F", "A", "S"]) expect(paintKey(k)).toBeNull();
    expect(paintKey("t")).toEqual({ sub: "terrain" });
  });

  it("t_and_k_are_the_smart_tools", () => {
    expect(paintKey("t")).toEqual({ sub: "terrain" });
    expect(paintKey("K")).toEqual({ sub: "cliff" });
    const text = (hints: { keys: string; does: string }[]) => hints.map((h) => `${h.keys} ${h.does}`).join(" · ");
    // Name and steps come from the state.
    expect(text(paintStatusHints({ ...BRUSH, sub: "terrain", edgeSet: "Weg" }))).toBe("Ziehen malt „Weg“ · ⇧Ziehen nimmt es weg");
    expect(text(paintStatusHints({ ...BRUSH, sub: "cliff", edgeSet: "Weg" }))).toBe("Ziehen hebt um 2 Stufen mit dem Rand „Weg“ · ⇧Ziehen senkt");
    expect(text(paintStatusHints({ ...BRUSH, sub: "cliff", edgeSet: "Fels", cliffRise: 1 }))).toBe("Ziehen hebt um 1 Stufe mit dem Rand „Fels“ · ⇧Ziehen senkt");
    // Without a set the status line asks for one.
    expect(text(paintStatusHints({ ...BRUSH, sub: "terrain" }))).toBe("Ziehen malt (erst einen Kantensatz wählen) · ⇧Ziehen nimmt es weg");
    expect(text(paintStatusHints({ ...BRUSH, sub: "cliff" }))).toBe("Ziehen hebt um 2 Stufen (erst einen Kantensatz wählen) · ⇧Ziehen senkt");
    // Z and X still turn and mirror the palette's tile, the sub-tool stays.
    expect(paintPatch({ ...BRUSH, sub: "terrain" }, { turn: true })).toEqual({ tile: "0041r1" });
  });

  it("status_hints_follow_the_sub_tool", () => {
    const text = (hints: { keys: string; does: string }[]) => hints.map((h) => `${h.keys} ${h.does}`).join(" · ");
    expect(text(paintStatusHints(BRUSH))).toBe("Ziehen malt · Rechtsklick nimmt auf · Z dreht · X spiegelt");
    expect(text(paintStatusHints({ ...BRUSH, sub: "height" }))).toBe("Klick hebt · ⇧Klick senkt · Ziehen über mehrere");
    expect(text(paintStatusHints({ ...BRUSH, sub: "ramp" }))).toBe("Klick setzt Rampe nach Osten, 1 Stufe · Z dreht · 1/2 Stufen · ⇧Klick entfernt");
    expect(text(paintStatusHints({ ...BRUSH, sub: "ramp", ramp: { dir: "n", steps: 2 } }))).toContain("Klick setzt Rampe nach Norden, 2 Stufen");
  });
});
