export const LAYOUT_KEY = "g4.layout.v2";

export type PanelId = "headers" | "world" | "matrix" | "inspector" | "status";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
  /** Initial width (left/right) or height (below) in px. */
  size?: number;
}

/**
 * Order matters: `world` is the anchor the other panels are placed around; `matrix` is a tab
 * of the world's group. The key is versioned: a layout stored by an older panel set is not read.
 */
export const DEFAULT_PANELS: readonly PanelSpec[] = [
  { id: "world", title: "Welt", position: { direction: "within" } },
  { id: "matrix", title: "Matrix", position: { direction: "within", referencePanel: "world" } },
  { id: "headers", title: "Header", position: { direction: "left", referencePanel: "world" }, size: 280 },
  { id: "inspector", title: "Inspector", position: { direction: "right", referencePanel: "world" }, size: 320 },
  { id: "status", title: "Status", position: { direction: "below", referencePanel: "world" }, size: 40 },
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
