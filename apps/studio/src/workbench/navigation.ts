import type { EventKind, HeaderSummary } from "../api";
import type { InspectorModel } from "./Inspector";
import type { PanelId } from "./layout";

/**
 * Where a link leads. `header` on a matrix cell, building or event is a header known to show it
 * (the cell's own header, say): navigation selects it first when the target is not shown yet.
 */
export type NavTarget =
  | { kind: "header"; id: number }
  | { kind: "matrixCell"; matrix: number; x: number; y: number; header?: number | null }
  | { kind: "area"; id: number }
  | { kind: "building"; map: number; index: number; header?: number | null }
  | { kind: "event"; eventFile: number; eventKind: EventKind; index: number; header?: number | null }
  /** A text archive, with one of its messages (omitted: the first). */
  | { kind: "textArchive"; index: number; message?: number }
  /** A script file (or level script), at a 1-based line (omitted: where the cursor was). */
  | { kind: "script"; file: number; line?: number };

/** What `nextSelection` needs to know about the current selection and the scene. */
export interface NavState {
  selectedHeader: number | null;
  /** The matrix the scene shows (null: none). */
  matrix: number | null;
  /** The maps the shown matrix places; omitted means none. */
  maps?: readonly number[];
  /** The event file the scene shows (the selected header's); omitted or null means none. */
  eventFile?: number | null;
  /** Every known header. When omitted, a header id is taken to exist. */
  headers?: readonly HeaderSummary[];
}

export interface NavResult {
  selectedHeader: number | null;
  /** What to pin in the inspector; null when the target does not exist or leaves the pin alone (a text archive). */
  pinned: InspectorModel | null;
  /** The panel that shows the target; null when nothing changes. */
  panel: PanelId | null;
  /** Set for a text archive: what the text panel opens (`message` null: the first one). */
  text?: { index: number; message: number | null };
  /** Set for a script: what the script panel opens (`line` null: keep the cursor). */
  script?: { file: number; line: number | null };
}

/**
 * The selection a link leads to, as a pure function (App applies it: selecting a header loads its
 * matrix, then the inspector is pinned and the panel focused). A header is only selected when the
 * target is a header, a matrix cell with a header, or a building or event that the shown scene
 * does not hold and a known header shows. A building or event the scene holds is just pinned.
 * Without a known header it is pinned alone: the editors read the backend, not the scene.
 */
export function nextSelection(state: NavState, target: NavTarget): NavResult {
  const known = (id: number | null | undefined): id is number => typeof id === "number" && (state.headers === undefined || state.headers.some((h) => h.id === id));
  const toHeader = (id: number): NavResult => ({ selectedHeader: id, pinned: { kind: "header", id }, panel: "headers" });
  // A header that the target needs selected first, unless it already is (null: keep the selection).
  const switchTo = (id: number | null | undefined): number | null => (known(id) && id !== state.selectedHeader ? id : null);

  switch (target.kind) {
    case "textArchive":
      // Neither the selection nor the pin changes: the text panel is not about a header.
      return { selectedHeader: state.selectedHeader, pinned: null, panel: "text", text: { index: target.index, message: target.message ?? null } };
    case "script":
      return { selectedHeader: state.selectedHeader, pinned: null, panel: "script", script: { file: target.file, line: target.line ?? null } };
    case "header":
      return known(target.id) ? toHeader(target.id) : { selectedHeader: state.selectedHeader, pinned: null, panel: null };
    case "matrixCell":
      if (known(target.header)) return toHeader(target.header);
      return { selectedHeader: state.selectedHeader, pinned: { kind: "matrixCell", matrix: target.matrix, x: target.x, y: target.y }, panel: "matrix" };
    case "area":
      return { selectedHeader: state.selectedHeader, pinned: { kind: "area", id: target.id }, panel: "inspector" };
    case "building": {
      const shown = (state.maps ?? []).includes(target.map);
      const header = shown ? null : switchTo(target.header);
      return {
        selectedHeader: header ?? state.selectedHeader,
        pinned: { kind: "building", map: target.map, index: target.index },
        panel: "world",
      };
    }
    case "event": {
      const shown = (state.eventFile ?? null) === target.eventFile;
      // The hint only helps when its header really reads that event file.
      const shows = (id: number) => state.headers === undefined || state.headers.find((h) => h.id === id)?.event_file_id === target.eventFile;
      const hint = target.header;
      const header = !shown && typeof hint === "number" && shows(hint) ? switchTo(hint) : null;
      return {
        selectedHeader: header ?? state.selectedHeader,
        pinned: { kind: "event", eventFile: target.eventFile, eventKind: target.eventKind, index: target.index },
        panel: "world",
      };
    }
  }
}
