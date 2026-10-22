export const LAYOUT_KEY = "g4.layout.v5";

export type PanelId = "headers" | "world" | "matrix" | "worldmap" | "inspector" | "text" | "script" | "problems" | "status";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
  /** Initial width (left/right) or height (below) in px. */
  size?: number;
}

/**
 * Order matters: `world` is the anchor the other panels are placed around; `matrix`, `worldmap`, `text` and `script`
 * are tabs of the world's group. `problems` is a group of its own between the world and the status strip: the strip
 * has no tab header and must stay alone in its group (`isSaneLayout`), so the list cannot be a tab of it. The explorer (`headers`, titled "Welt") is the left panel. The key is versioned: a layout stored by an older panel set is not read.
 */
export const DEFAULT_PANELS: readonly PanelSpec[] = [
  { id: "world", title: "3D", position: { direction: "within" } },
  { id: "matrix", title: "Raster", position: { direction: "within", referencePanel: "world" } },
  { id: "worldmap", title: "Weltkarte", position: { direction: "within", referencePanel: "world" } },
  { id: "text", title: "Text", position: { direction: "within", referencePanel: "world" } },
  { id: "script", title: "Script", position: { direction: "within", referencePanel: "world" } },
  { id: "headers", title: "Welt", position: { direction: "left", referencePanel: "world" }, size: 300 },
  { id: "inspector", title: "Inspector", position: { direction: "right", referencePanel: "world" }, size: 320 },
  { id: "status", title: "Status", position: { direction: "below", referencePanel: "world" }, size: 40 },
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

/** The panel-id lists ("views") of every group in a serialized dockview layout (grid, floating, popout). */
function groupViews(node: unknown, found: string[][] = []): string[][] {
  if (Array.isArray(node)) {
    node.forEach((child) => groupViews(child, found));
  } else if (typeof node === "object" && node !== null) {
    const record = node as Record<string, unknown>;
    const views = record.views;
    if (Array.isArray(views) && views.every((v) => typeof v === "string")) found.push(views as string[]);
    Object.values(record).forEach((child) => groupViews(child, found));
  }
  return found;
}

/**
 * True when a serialized dockview layout can be restored: every default panel exists and the
 * status panel (a fixed strip without tab header) is alone in its group.
 */
export function isSaneLayout(layout: object): boolean {
  const panels = (layout as { panels?: unknown }).panels;
  if (typeof panels !== "object" || panels === null) return false;
  if (!DEFAULT_PANELS.every((p) => Object.prototype.hasOwnProperty.call(panels, p.id))) return false;
  const statusGroup = groupViews(layout).find((views) => views.includes("status"));
  return statusGroup !== undefined && statusGroup.length === 1;
}
