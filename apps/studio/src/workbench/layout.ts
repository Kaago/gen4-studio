export const LAYOUT_KEY = "g4.layout.v5";

export type PanelId = "headers" | "world" | "matrix" | "worldmap" | "inspector" | "text" | "script" | "problems";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
  /** Initial width (left/right) or height (below) in px. */
  size?: number;
}

/**
 * Order matters: `world` is the anchor the other panels are placed around; `matrix`, `worldmap`, `text` and `script`
 * are tabs of the world's group. `problems` is a group of its own below them. The explorer (`headers`, titled "Welt") is the left
 * panel. The key is versioned: a layout stored by an older panel set is not read. It stays v5 while the status strip is gone (the
 * status line lives in the frame now): a v5 layout that still holds `status` is rejected by `isSaneLayout` and replaced by the default.
 */
export const DEFAULT_PANELS: readonly PanelSpec[] = [
  { id: "world", title: "3D", position: { direction: "within" } },
  { id: "matrix", title: "Raster", position: { direction: "within", referencePanel: "world" } },
  { id: "worldmap", title: "Weltkarte", position: { direction: "within", referencePanel: "world" } },
  { id: "text", title: "Text", position: { direction: "within", referencePanel: "world" } },
  { id: "script", title: "Script", position: { direction: "within", referencePanel: "world" } },
  { id: "headers", title: "Welt", position: { direction: "left", referencePanel: "world" }, size: 300 },
  { id: "inspector", title: "Inspector", position: { direction: "right", referencePanel: "world" }, size: 320 },
  { id: "problems", title: "Probleme", position: { direction: "below", referencePanel: "world" }, size: 160 },
];

export function saveLayout(json: object): void {
  try {
    localStorage.setItem(LAYOUT_KEY, JSON.stringify(json));
  } catch {
    /* the layout is simply not remembered */
  }
}

export function loadLayout(): object | null {
  try {
    const raw = localStorage.getItem(LAYOUT_KEY);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * True when a serialized dockview layout can be restored: every default panel exists and no other panel does (a layout
 * stored by an older panel set, such as one with the removed status strip, would bring back a panel that has no component).
 */
export function isSaneLayout(layout: object): boolean {
  const panels = (layout as { panels?: unknown }).panels;
  if (typeof panels !== "object" || panels === null) return false;
  const known = new Set<string>(DEFAULT_PANELS.map((p) => p.id));
  if (!DEFAULT_PANELS.every((p) => Object.prototype.hasOwnProperty.call(panels, p.id))) return false;
  return Object.keys(panels).every((id) => known.has(id));
}

