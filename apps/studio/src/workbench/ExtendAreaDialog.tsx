import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { extendArea } from "../api";
import type { ExtendReport, HeaderSummary, MatrixDto, MemberCounts } from "../api";
import { DraftInput } from "./DraftInput";
import { extendDefaults, extendSpec } from "./extend";
import { parseCellField } from "./matrix";
import { id4 } from "./terms";

interface Props {
  matrix: MatrixDto;
  x: number;
  y: number;
  headers: HeaderSummary[];
  counts: MemberCounts;
  /** The area was extended (`report`), or the dialog was left without a result (null: cancelled, or it failed, see `onError`). */
  onDone: (report: ExtendReport | null) => void;
  onError: (e: unknown) => void;
}

const backdrop: CSSProperties = { position: "fixed", inset: 0, background: "rgba(0,0,0,0.35)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 50 };
const sheet: CSSProperties = { background: "#fff", borderRadius: 8, padding: 16, width: 380, maxWidth: "90vw", fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" };
const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "6px 0" };
const field: CSSProperties = { width: 72, textAlign: "right", fontFamily: "ui-monospace, monospace" };

/** Creates a map copy at the cell `(x, y)` of a matrix, growing the grid when the cell lies outside it. */
export function ExtendAreaDialog({ matrix, x, y, headers, counts, onDone, onError }: Props) {
  const defaults = extendDefaults(matrix, x, y);
  const neighbour = defaults.neighbour;
  const mapMax = Math.max(0, (counts.Map ?? 1) - 1);
  const hasHeaders = matrix.headers != null;
  const headerMax = Math.max(0, headers.length - 1);

  const [mode, setMode] = useState<"neighbour" | "pick">(neighbour ? "neighbour" : "pick");
  const [picked, setPicked] = useState(neighbour?.map ?? 0);
  const [clearBuildings, setClearBuildings] = useState(false);
  const [headerCell, setHeaderCell] = useState(neighbour?.header ?? 0);
  const [working, setWorking] = useState(false);

  // Escape cancels (not while the request runs) and the first control has the focus. Escape in a text
  // field takes back what was typed there (see DraftInput); it must not also close the dialog.
  const dialog = useRef<HTMLDivElement>(null);
  const leave = useRef(onDone);
  leave.current = onDone;
  const running = useRef(working);
  running.current = working;
  useEffect(() => {
    dialog.current?.querySelector<HTMLElement>("input, select, button")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || running.current) return;
      if (e.target instanceof HTMLElement && e.target.matches("textarea, select, input:not([type=radio]):not([type=checkbox])")) return;
      leave.current(null);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  const sourceMap = mode === "neighbour" && neighbour ? neighbour.map : picked;

  async function create() {
    setWorking(true);
    let report: ExtendReport;
    try {
      report = await extendArea(extendSpec(matrix, x, y, { map: sourceMap, clearBuildings }, headerCell));
    } catch (e) {
      onError(e);
      onDone(null);
      return;
    }
    onDone(report);
  }

  return (
    <div style={backdrop} role="presentation">
      <div ref={dialog} role="dialog" aria-modal="true" aria-label={`Karte bei x ${x}, y ${y} anlegen`} style={sheet}>
        <div style={{ fontSize: 14, fontWeight: 600 }}>Karte bei x {x}, y {y} anlegen</div>
        <fieldset style={{ border: "none", padding: 0, margin: "8px 0" }} disabled={working}>
          {neighbour && (
            <label style={row}>
              <input type="radio" name="source" checked={mode === "neighbour"} onChange={() => setMode("neighbour")} />
              Kopie von Karte {id4(neighbour.map)} (Nachbarzelle)
            </label>
          )}
          <label style={row}>
            <input type="radio" name="source" checked={mode === "pick"} onChange={() => setMode("pick")} />
            Karte wählen
            <DraftInput
              label={`Karte (0 bis ${mapMax})`}
              type="number"
              min={0}
              max={mapMax}
              value={String(picked)}
              onCommit={(text) => {
                const n = parseCellField(text, mapMax, false);
                if (typeof n === "number") {
                  setPicked(n);
                  setMode("pick");
                }
              }}
              style={field}
            />
          </label>
          <label style={row}>
            <input type="checkbox" checked={clearBuildings} onChange={(e) => setClearBuildings(e.target.checked)} />
            ohne Gebäude
          </label>
          {hasHeaders && (
            <label style={row}>
              Header-Zelle
              <DraftInput
                label={`Header-Zelle (0 bis ${headerMax})`}
                type="number"
                min={0}
                max={headerMax}
                value={String(headerCell)}
                onCommit={(text) => {
                  const n = parseCellField(text, headerMax, false);
                  if (typeof n === "number") setHeaderCell(n);
                }}
                style={field}
              />
            </label>
          )}
        </fieldset>
        <p style={{ margin: "8px 0", color: "#6b7280" }}>
          {defaults.width > matrix.width || defaults.height > matrix.height
            ? `Das Raster wird auf ${defaults.width} × ${defaults.height} vergrößert.`
            : `Das Raster bleibt ${defaults.width} × ${defaults.height} groß.`}{" "}
          Anlegen lässt sich nicht rückgängig machen.
        </p>
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button onClick={() => onDone(null)} disabled={working}>
            Abbrechen
          </button>
          <button onClick={() => void create()} disabled={working}>
            Anlegen
          </button>
        </div>
      </div>
    </div>
  );
}
