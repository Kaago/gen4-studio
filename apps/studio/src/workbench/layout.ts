export const LAYOUT_KEY = "g4.layout.v1";

export type PanelId = "headers" | "world" | "inspector" | "status";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
  /** Initial width (left/right) or height (below) in px. */
  size?: number;
}

/** Order matters: `world` is the anchor the other panels are placed around. */
export const DEFAULT_PANELS: readonly PanelSpec[] = [
  { id: "world", title: "Welt", position: { direction: "within" } },
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

/** True when a serialized dockview layout still contains every default panel. */
export function hasAllPanels(layout: object): boolean {
  const panels = (layout as { panels?: unknown }).panels;
  if (typeof panels !== "object" || panels === null) return false;
  return DEFAULT_PANELS.every((p) => Object.prototype.hasOwnProperty.call(panels, p.id));
}
