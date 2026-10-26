import { useState } from "react";
import { extendArea } from "../api";
import type { ExtendReport, HeaderSummary, MatrixDto, MemberCounts } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { Badge, Checkbox, Dialog, NumberField, Segmented } from "../ui/primitives";
import { extendDefaults, extendSpec } from "./extend";
import styles from "./ExtendAreaDialog.module.css";
import { id4 } from "./terms";

interface Props {
  matrix: MatrixDto;
  x: number;
  y: number;
  /** The cell whose map and header the dialog proposes (the area's cell beside a "+" of the world map); default: see `extendDefaults`. */
  prefer?: { x: number; y: number } | null;
  /** The name of the area or room the new map belongs to: Bauen says so instead of showing its number. */
  areaName?: string | null;
  /**
   * The header of that area or room (the one being edited). Bauen proposes it when the proposed cell is landscape (header 0), so the
   * sentence and the request agree; without it, a landscape neighbour makes Bauen ask for the number.
   */
  areaHeader?: number | null;
  /** Bauen speaks of maps and areas, never of the grid or headers; default Tüfteln (the Raster tab). */
  depth?: Depth;
  headers: HeaderSummary[];
  counts: MemberCounts;
  /** The area was extended (`report`), or the dialog was left without a result (null: cancelled, or it failed, see `onError`). */
  onDone: (report: ExtendReport | null) => void;
  onError: (e: unknown) => void;
}

/** What happens to the size of the grid: Bauen says "Die Karte wird größer", Tüfteln names the Raster. */
function sizeNote(building: boolean, matrix: MatrixDto, width: number, height: number): string {
  const grows = width > matrix.width || height > matrix.height;
  if (building) return grows ? `Die Karte wird größer: ${width} × ${height}.` : `Die Größe bleibt ${width} × ${height}.`;
  return grows ? `Das Raster wird auf ${width} × ${height} vergrößert.` : `Das Raster bleibt ${width} × ${height} groß.`;
}

/**
 * Creates a map copy at the cell `(x, y)` of a matrix, growing the grid when the cell lies outside it. A `Dialog`: the focus
 * stays inside, Escape and the cross cancel (not while the request runs; Escape that takes back typed text does not close it).
 */
export function ExtendAreaDialog({ matrix, x, y, prefer = null, areaName = null, areaHeader = null, depth = "tuefteln", headers, counts, onDone, onError }: Props) {
  const building = depth === "bauen";
  const defaults = extendDefaults(matrix, x, y, prefer);
  const neighbour = defaults.neighbour;
  const mapMax = Math.max(0, (counts.Map ?? 1) - 1);
  const hasHeaders = matrix.headers != null;
  const headerMax = Math.max(0, headers.length - 1);

  const [mode, setMode] = useState<"neighbour" | "pick">(neighbour ? "neighbour" : "pick");
  const [picked, setPicked] = useState(neighbour?.map ?? 0);
  const [clearBuildings, setClearBuildings] = useState(false);
  // Bauen: a landscape neighbour (header 0) does not decide where the new map belongs; the edited area or room does.
  const proposedHeader = building && neighbour?.header === 0 && areaHeader != null ? areaHeader : (neighbour?.header ?? 0);
  // The sentence only names a place the map really goes to: a neighbour's header other than the landscape.
  const belongs = building && neighbour?.header != null && proposedHeader !== 0;
  const [headerCell, setHeaderCell] = useState(proposedHeader);
  const [working, setWorking] = useState(false);

  const sourceMap = mode === "neighbour" && neighbour ? neighbour.map : picked;

  async function create() {
    if (working) return;
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

  const leave = () => {
    if (!working) onDone(null);
  };
  const sources = [
    ...(neighbour ? [{ value: "neighbour" as const, label: `Kopie von Karte ${id4(neighbour.map)} (Nachbarzelle)` }] : []),
    { value: "pick" as const, label: "Karte wählen" },
  ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title={`Karte bei x ${x}, y ${y} anlegen?`}
      description="Anlegen lässt sich nicht rückgängig machen."
      actions={[
        { label: "Abbrechen", onClick: leave },
        { label: "Karte anlegen", kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.form} aria-busy={working}>
        <Badge kind="irreversible" />
        {sources.length > 1 && <Segmented ariaLabel="Quelle der Kopie" options={sources} value={mode} onChange={setMode} />}
        {mode === "pick" && (
          <NumberField label="Karte" showLabel hint={`0 bis ${mapMax}`} min={0} max={mapMax} value={picked} disabled={working} onCommit={setPicked} />
        )}
        <Checkbox label="ohne Gebäude" checked={clearBuildings} disabled={working} onChange={setClearBuildings} />
        {hasHeaders &&
          (belongs ? (
            <p className={styles.size}>Die neue Karte gehört zu „{areaName ?? id4(headerCell)}“.</p>
          ) : (
            <NumberField
              label={building ? "Nummer des Gebiets" : "Header-Zelle"}
              showLabel
              hint={`0 bis ${headerMax}`}
              min={0}
              max={headerMax}
              value={headerCell}
              disabled={working}
              onCommit={setHeaderCell}
            />
          ))}
        <p className={styles.size}>{sizeNote(building, matrix, defaults.width, defaults.height)}</p>
      </div>
    </Dialog>
  );
}
