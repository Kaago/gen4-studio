import { describe, expect, it } from "vitest";
import {
  buildingFallbackBox,
  buildingKey,
  buildingModelIds,
  buildingTransform,
  isTypingTarget,
  deletePrompt,
  newBuildingPosition,
  reconcileBuildings,
  sameBuildings,
  settleBuilding,
  SHORTCUTS,
  shortcutAction,
  snapHalfTile,
  wantsDelete,
} from "./buildings";

const b = (model_id: number) => ({ model_id, x: 0, y: 0, z: 0, sx: 1, sy: 1, sz: 1, rotation: ["00000000", "00000000", "00000000"] as [string, string, string] });

describe("buildings", () => {
  it("places a building model with the map scale times its own scale", () => {
    const t = buildingTransform({ model_id: 1, x: -4, y: 0, z: 6, sx: 2, sy: 1, sz: 3 }, { x: 64, z: 32 });
    expect(t.position).toEqual([76, 0, 54]);
    expect(t.scale).toEqual([2 / 16, 1 / 16, 3 / 16]);
  });

  it("adds the height of the cell to the building's own y", () => {
    expect(buildingTransform({ model_id: 1, x: 0, y: 0.5, z: 0, sx: 1, sy: 1, sz: 1 }, { x: 0, y: 1, z: 0 }).position[1]).toBe(1.5);
    expect(buildingTransform({ model_id: 1, x: 0, y: 0.5, z: 0, sx: 1, sy: 1, sz: 1 }, { x: 0, z: 0 }).position[1]).toBe(0.5);
  });

  it("lifts the placeholder box by half its height, sized by the scale factors", () => {
    expect(buildingFallbackBox({ x: 0, y: 2, z: 0, sx: 2, sy: 1, sz: 3 })).toEqual({ offsetY: 0.5, size: [2, 1, 3] });
  });

  it("snaps to half tiles", () => {
    expect(snapHalfTile(1.26)).toBe(1.5);
    expect(snapHalfTile(1.24)).toBe(1);
    expect(snapHalfTile(-0.3)).toBe(-0.5);
    expect(Object.is(snapHalfTile(-0.2), 0)).toBe(true);
  });

  it("keys a building by its map and index", () => {
    expect(buildingKey(12, 3)).toBe("12:3");
  });

  it("lists each model id once, in order of first use", () => {
    expect(buildingModelIds([b(7), b(3), b(7), b(0)])).toEqual([7, 3, 0]);
    expect(buildingModelIds([])).toEqual([]);
  });

  it("puts a new building at the centre of the hovered tile", () => {
    // Tile (16, 16) is the first one right/below the map centre.
    expect(newBuildingPosition(16, 16)).toEqual({ x: 0.5, z: 0.5 });
    expect(newBuildingPosition(0, 31)).toEqual({ x: -15.5, z: 15.5 });
  });

  it("does not treat keys typed into form fields as scene shortcuts", () => {
    expect(isTypingTarget({ tagName: "INPUT" })).toBe(true);
    expect(isTypingTarget({ tagName: "TEXTAREA" })).toBe(true);
    expect(isTypingTarget({ tagName: "SELECT" })).toBe(true);
    expect(isTypingTarget({ tagName: "DIV", isContentEditable: true })).toBe(true);
    expect(isTypingTarget({ tagName: "CANVAS" })).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });

  it("deletes on Delete or Backspace, but never while typing or with a modifier", () => {
    const key = (k: string, extra: object = {}) => ({ key: k, target: { tagName: "BODY" }, metaKey: false, ctrlKey: false, altKey: false, ...extra });
    expect(wantsDelete(key("Delete"))).toBe(true);
    expect(wantsDelete(key("Backspace"))).toBe(true);
    expect(wantsDelete(key("a"))).toBe(false);
    expect(wantsDelete(key("Delete", { target: { tagName: "INPUT" } }))).toBe(false);
    expect(wantsDelete(key("Delete", { metaKey: true }))).toBe(false);
  });

  it("compares building lists field by field", () => {
    expect(sameBuildings([b(1), b(2)], [b(1), b(2)])).toBe(true);
    expect(sameBuildings([b(1)], [b(1), b(2)])).toBe(false);
    expect(sameBuildings([b(1)], [b(2)])).toBe(false);
    expect(sameBuildings([b(1)], [{ ...b(1), z: 0.5 }])).toBe(false);
    expect(sameBuildings([b(1)], [{ ...b(1), rotation: ["00000000", "00000001", "00000000"] }])).toBe(false);
    expect(sameBuildings([], [])).toBe(true);
  });

  it("keeps the old building objects when a reload brings nothing new", () => {
    const current = [b(1), b(2)];
    const r = reconcileBuildings(current, [b(1), b(2)]);
    expect(r.changed).toBe(false);
    expect(r.buildings).toBe(current);
    const fresh = [b(1), b(3)];
    expect(reconcileBuildings(current, fresh)).toEqual({ buildings: fresh, changed: true });
  });

  it("settles a move by value, so a reload in between does not lose it", () => {
    const list = [b(1), b(2)];
    // The scene's slot holds `slot`; after a reload without changes the list still holds it.
    const slot = list[1];
    const moved = { ...slot, x: 2.5 };
    expect(settleBuilding(list, 1, slot, moved)).toBe(true);
    expect(list[1]).toBe(moved);
    // Even a fresh object with the same values counts as the same building.
    expect(settleBuilding(list, 0, { ...b(1) }, { ...b(1), z: 1 })).toBe(true);
    // A building that changed meanwhile, or is gone, is left alone.
    expect(settleBuilding(list, 1, slot, { ...slot, x: 9 })).toBe(false);
    expect(settleBuilding(list, 5, slot, slot)).toBe(false);
    expect(list[1]).toBe(moved);
  });

  it("follows the repro: undo after a reload, a drag, and another undo", () => {
    // Map data and slot share the objects; this mirrors MatrixScene's bookkeeping.
    let data = [b(1)];
    let slot = data[0];
    let rebuilt = 0;
    const reload = (fresh: ReturnType<typeof b>[]) => {
      const r = reconcileBuildings(data, fresh);
      data = r.buildings;
      if (r.changed) {
        slot = data[0];
        rebuilt++;
      }
    };
    reload([b(1)]); // a tile paint undone: nothing changed in the buildings
    expect(rebuilt).toBe(0);
    const moved = { ...slot, x: 1 };
    expect(settleBuilding(data, 0, slot, moved)).toBe(true); // the drag is written
    slot = moved; // layer.show(ref, moved)
    reload([b(1)]); // Cmd+Z: the backend has the old position again
    expect(rebuilt).toBe(1);
    expect(slot.x).toBe(0);
    expect(data[0].x).toBe(0);
  });

  it("names model and position in the delete confirmation", () => {
    expect(deletePrompt(12, 3, { model_id: 7, x: -5.5, y: 1, z: 4 })).toBe("Gebäude 4 von Karte 0012 (Modell 7, x -5.5, y 1, z 4) löschen?");
    expect(deletePrompt(12, 3, undefined)).toBe("Gebäude 4 von Karte 0012 löschen?");
  });

  it("maps Cmd/Ctrl+Z and +S to app shortcuts, but leaves Z to a field being typed in", () => {
    const key = (k: string, o: Partial<{ target: { tagName?: string } | null; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) =>
      shortcutAction({ key: k, target: { tagName: "CANVAS" }, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...o });
    expect(key("z")).toBe("undo");
    expect(key("Z", { shiftKey: true })).toBe("redo");
    expect(key("z", { metaKey: false, ctrlKey: true })).toBe("undo");
    expect(key("s")).toBe("save");
    expect(key("z", { target: null })).toBe("undo");
    // Typing: the text field keeps its own undo; saving still works.
    expect(key("z", { target: { tagName: "INPUT" } })).toBeNull();
    expect(key("z", { target: { tagName: "TEXTAREA" }, shiftKey: true })).toBeNull();
    expect(key("s", { target: { tagName: "INPUT" } })).toBe("save");
    // Unmodified or Alt-modified keys are not shortcuts.
    expect(key("z", { metaKey: false })).toBeNull();
    expect(key("z", { altKey: true })).toBeNull();
    expect(key("x")).toBeNull();
  });

  it("maps Cmd/Ctrl+R to playing inside the app only, so a browser keeps its reload", () => {
    const key = (k: string, o: Partial<{ inApp: boolean; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; target: { tagName?: string } | null }> = {}) =>
      shortcutAction({ key: k, target: { tagName: "CANVAS" }, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...o });
    expect(key("r", { inApp: true })).toBe("play");
    expect(key("R", { inApp: true, metaKey: false, ctrlKey: true })).toBe("play");
    // Typing in a field does not stop it, like saving.
    expect(key("r", { inApp: true, target: { tagName: "INPUT" } })).toBe("play");
    expect(key("r")).toBeNull();
    expect(key("r", { inApp: false })).toBeNull();
    // The hard reload, an Alt chord and an unmodified R are not ours.
    expect(key("r", { inApp: true, shiftKey: true })).toBeNull();
    expect(key("r", { inApp: true, altKey: true })).toBeNull();
    expect(key("r", { inApp: true, metaKey: false })).toBeNull();
  });

  it("maps Cmd/Ctrl+O to opening a project, also while typing, but not with Shift or Alt", () => {
    const key = (k: string, o: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; target: { tagName?: string } | null }> = {}) =>
      shortcutAction({ key: k, target: { tagName: "CANVAS" }, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...o });
    expect(key("o")).toBe("open");
    expect(key("O", { metaKey: false, ctrlKey: true })).toBe("open");
    expect(key("o", { target: { tagName: "INPUT" } })).toBe("open");
    expect(key("o", { shiftKey: true })).toBeNull();
    expect(key("o", { altKey: true })).toBeNull();
    expect(key("o", { metaKey: false })).toBeNull();
  });

  it("maps Cmd/Ctrl+K to the palette, also while typing, but not with Shift or Alt", () => {
    const key = (k: string, o: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; target: { tagName?: string } | null }> = {}) =>
      shortcutAction({ key: k, target: { tagName: "CANVAS" }, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...o });
    expect(key("k")).toBe("palette");
    expect(key("K", { metaKey: false, ctrlKey: true })).toBe("palette");
    expect(key("k", { target: { tagName: "INPUT" } })).toBe("palette");
    expect(key("k", { shiftKey: true })).toBeNull();
    expect(key("k", { altKey: true })).toBeNull();
    expect(key("k", { metaKey: false })).toBeNull();
  });

  it("lists every shortcut with keys that shortcutAction maps back to it", () => {
    // The list in the settings cannot drift: each entry's keys, pressed inside the app, ask for that entry's action.
    for (const [action, { keys, label }] of Object.entries(SHORTCUTS)) {
      expect(label.length).toBeGreaterThan(0);
      const key = keys.replace(/[⌘⇧]/g, "").toLowerCase();
      const shiftKey = keys.includes("⇧");
      expect(keys.startsWith("⌘") || keys.startsWith("⇧⌘")).toBe(true);
      expect(shortcutAction({ key, target: { tagName: "CANVAS" }, metaKey: true, ctrlKey: false, altKey: false, shiftKey, inApp: true })).toBe(action);
    }
    expect(Object.keys(SHORTCUTS).sort()).toEqual(["open", "palette", "play", "redo", "save", "undo"]);
  });
});
