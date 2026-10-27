import { useEffect, useMemo, useState } from "react";
import { MATRIX_EMPTY, duplicateMap, setMatrixCell } from "../../api";
import type { AssetId, HeaderSummary, MemberCounts } from "../../api";
import type { ChecksApi } from "../../checks/useChecks";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import type { Depth } from "../../ui/frame/shellState";
import { Button, InspectorRow, NumberField, Picker, Reference, TextField, checkNumber } from "../../ui/primitives";
import type { MenuEntry, PickerItem } from "../../ui/primitives";
import type { HeaderIdentity } from "../headerIdentity";
import { cellLabel } from "../matrix";
import type { NavTarget } from "../navigation";
import { TERMS, id4, termFor } from "../terms";
import { useMatrix } from "../useMatrix";
import { useWorld } from "../WorkbenchContext";
import type { World } from "../worldModel";
import styles from "./Editor.module.css";
import { ExportPartDialog } from "./ExportPartDialog";
import { ImportPartsDialog } from "./ImportPartsDialog";
import { isDirty } from "./dirtyAssets";
import { InspectorHead } from "./InspectorHead";
import { RawRowSection } from "./RawRowSection";
import { TechnicalSection } from "./TechnicalSection";

/** What the cell editor needs from App. */
export interface MatrixCellContext {
  /** The assets with unsaved changes: the head says "Ungespeichert" while the matrix is one. */
  dirty: readonly AssetId[];
  /** The headers, and what the world tree calls each (`WorkbenchWorld.identify`): Bauen names the area or room of a cell. */
  headers: readonly HeaderSummary[];
  identify: (header: HeaderSummary) => HeaderIdentity;
  /** Number of map headers (the largest header id plus one). */
  headerCount: number;
  /** Follows a link (the cell's header). */
  goTo: (target: NavTarget) => void;
  counts: MemberCounts;
  /** Changes whenever matrix data may have changed behind the editor (edits, undo, redo, reload). */
  revision: number;
  onEdited: (index: number) => void | Promise<void>;
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
  /** Parts were imported into a map: pictures, scene and dirty count catch up. */
  onMapImported: (map: number) => void | Promise<void>;
  /** The checks of the project: the import dialog lists what they find at the map after an import. */
  checks: ChecksApi | null;
}

/** The map of a cell: a number from 0 to `max`, or nothing for a cell without a map. An entry outside that is refused with its limit. */
function MapField({ map, max, onCommit }: { map: number | null; max: number; onCommit: (next: number | "empty") => void }) {
  const [error, setError] = useState<string | undefined>();
  // A new value from the matrix makes an old refusal moot.
  useEffect(() => setError(undefined), [map]);
  return (
    <TextField
      label="Karte"
      placeholder="leer"
      value={map === null ? "" : String(map)}
      error={error}
      onChange={() => setError(undefined)}
      onCommit={(text) => {
        if (text.trim() === "") {
          setError(undefined);
          return onCommit("empty");
        }
        const checked = checkNumber(text, "dec", 0, max);
        if (!checked.ok) return setError(checked.error);
        setError(undefined);
        onCommit(checked.value);
      }}
    />
  );
}

/** Header 0 fills the landscape cells of a world map: no area, no place. */
const LANDSCAPE = 0;

/** The areas whose cells lie on matrix `index` (a world map, both halves of a split one, or an area with its own map). */
function areasOn(world: World | null, index: number) {
  if (world === null) return [];
  return [...world.worldMaps.flatMap((m) => m.areas), ...world.moreAreas].filter((a) => a.matrix === index);
}

/**
 * Bauen's name of a matrix: the world map(s) on it („Johto und Kanto“ for the split one), else the area or room it belongs to, else
 * „Raumplan“ (never a matrix number).
 */
function matrixWords(world: World | null, index: number, headers: readonly HeaderSummary[], identify: (header: HeaderSummary) => HeaderIdentity): string {
  const maps = world?.worldMaps.filter((m) => m.matrix === index) ?? [];
  if (maps.length > 0) return maps.map((m) => m.name).join(" und ");
  const own = world?.moreAreas.find((a) => a.matrix === index);
  if (own) return own.name;
  const user = headers.find((h) => h.matrix_id === index);
  return user ? identify(user).name : "Raumplan";
}

/**
 * Bauen's row „Gehört zu“ of a cell on a map with areas: a reference to the area of the cell (none for the landscape) and „Ändern…“,
 * a picker of „Landschaft“ and the areas of this map (an area with several sections lists each). Tüfteln keeps the header number.
 */
function OwnerField({
  matrix,
  header,
  headers,
  identify,
  goTo,
  onCommit,
}: {
  matrix: number;
  header: number;
  headers: readonly HeaderSummary[];
  identify: (header: HeaderSummary) => HeaderIdentity;
  goTo: (target: NavTarget) => void;
  onCommit: (header: number) => void;
}) {
  const { world } = useWorld();
  const [picking, setPicking] = useState(false);
  const items = useMemo<PickerItem[]>(() => {
    const seen = new Set<number>([LANDSCAPE]);
    const areas: PickerItem[] = [];
    for (const area of areasOn(world, matrix)) {
      area.headers.forEach((h, i) => {
        if (seen.has(h.id)) return;
        seen.add(h.id);
        areas.push({ id: h.id, label: area.headers.length > 1 ? `${area.name}, Abschnitt ${i + 1}` : area.name });
      });
    }
    return [{ id: LANDSCAPE, label: TERMS.landscape.label, description: TERMS.landscape.short }, ...areas];
  }, [world, matrix]);
  const owner = header === LANDSCAPE ? undefined : headers.find((h) => h.id === header);
  return (
    <>
      <InspectorRow label="Gehört zu">
        <div className={styles.inline}>
          {header === LANDSCAPE ? (
            <span>{TERMS.landscape.label}</span>
          ) : (
            <Reference area="welt" label={owner ? identify(owner).name : `Nummer ${id4(header)}`} onOpen={() => goTo({ kind: "header", id: header })} />
          )}
          <Button kind="quiet" size="s" aria-label="Gehört zu ändern…" data-cell-owner="" onClick={() => setPicking(true)}>
            Ändern…
          </Button>
        </div>
      </InspectorRow>
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title="Gehört zu"
        items={items}
        placeholder="Gebiet suchen …"
        emptyText="Kein Gebiet passt zur Suche."
        onPick={(item) => Number(item.id) !== header && onCommit(Number(item.id))}
      />
    </>
  );
}

/** Editor of one matrix cell: map, header and altitude, each one undoable change. */
export function MatrixCellEditor({ matrix: index, x, y, context, depth }: { matrix: number; x: number; y: number; context: MatrixCellContext; depth: Depth }) {
  const { dirty, headers, identify, headerCount, goTo, counts, revision, onEdited, onCountsChanged, onError, onMapImported, checks } = context;
  const matrix = useMatrix(index, revision, onError);
  const { world } = useWorld();
  const reset = useReset();
  const [working, setWorking] = useState(false);
  const [partsDialog, setPartsDialog] = useState<"import" | "export" | null>(null);

  const cell = `Zelle ${x}, ${y}`;
  // Bauen speaks of the world map and the area a cell belongs to; Tüfteln keeps the matrix ("Raster") and the header.
  const tinker = depth === "tuefteln";
  // Bauen names the map by its world map, area or room, never by its number (a cell outside or a matrix still loading).
  const matrixName = tinker ? `${TERMS.matrix.label} ${id4(index)}` : matrixWords(world, index, headers, identify);
  const headerWord = termFor("header", depth, "area");
  const unsaved = isDirty(dirty, "Matrix", index);
  if (!matrix) {
    return (
      <div className={styles.panel}>
        <InspectorHead kind="map" title={cell} ident={matrixName} unsaved={unsaved} depth={depth} />
      </div>
    );
  }
  if (x >= matrix.width || y >= matrix.height) {
    return (
      <div className={styles.panel}>
        <InspectorHead kind="map" title={cell} ident={matrixName} unsaved={unsaved} depth={depth} />
        <p className={styles.note}>
          Die Zelle liegt außerhalb {tinker ? "des Rasters" : "der Karten"} ({matrix.width} × {matrix.height}).
        </p>
      </div>
    );
  }
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  const header = matrix.headers ? (matrix.headers[i] ?? 0) : null;
  const altitude = matrix.altitudes ? (matrix.altitudes[i] ?? 0) : null;
  const mapMax = Math.max(0, (counts.Map ?? 1) - 1);
  const headerMax = Math.max(0, headerCount - 1);
  // Bauen names the cell by its map and where it lies (the area or room); Tüfteln keeps the codes of the matrix and the header.
  // Header 0 is the landscape, not a place a cell lies in.
  const place = header === null || header === 0 ? undefined : headers.find((h) => h.id === header);
  const title = tinker ? cell : map === null ? "Leere Zelle" : `Karte ${id4(map)}`;
  const ident = tinker ? `${matrixName} · ${cellLabel(map, header)}` : place ? `${cell} · ${identify(place).name}` : cell;

  /** Writes the changed field of this cell; always refreshes afterwards, also after a refusal. */
  async function run(op: () => Promise<unknown>, alsoCounts = false) {
    setWorking(true);
    try {
      await op();
    } catch (e) {
      onError(e);
    } finally {
      try {
        if (alsoCounts) await onCountsChanged();
        await onEdited(index);
      } finally {
        setWorking(false);
      }
    }
  }

  const commitMap = (v: number | "empty") => {
    const next = v === "empty" ? MATRIX_EMPTY : v;
    if (next !== (map ?? MATRIX_EMPTY)) void run(() => setMatrixCell(index, x, y, next, null, null));
  };
  const commitHeader = (v: number) => void run(() => setMatrixCell(index, x, y, null, v, null));
  const commitAltitude = (v: number) => void run(() => setMatrixCell(index, x, y, null, null, v));

  // The copy is appended first; if placing it fails, the (already written) copy is still reported through the counts.
  const duplicate = (source: number) =>
    run(async () => {
      const copy = await duplicateMap(source);
      await setMatrixCell(index, x, y, copy, null, null);
    }, true);

  // The map's actions: its parts in and out (only when the cell has a map), a copy of it in this cell, and the map as in the base ROM (maps
  // have no list: this head is the one place to reset one).
  const actions: MenuEntry[] = [
    ...(map === null
      ? []
      : [
          { label: "Teile importieren…", onSelect: () => setPartsDialog("import") },
          { label: "Teile exportieren…", onSelect: () => setPartsDialog("export") },
        ]),
    { label: "Duplizieren", disabled: map === null || working, onSelect: () => map !== null && void duplicate(map) },
    ...(map === null ? [] : ["separator" as const, resetEntry({ kind: "map", index: map }, reset.status, reset.open)]),
  ];

  return (
    <div className={styles.panel}>
      <InspectorHead kind="map" title={title} ident={ident} unsaved={unsaved} actions={actions} depth={depth} />
      <div className={styles.body}>
        <InspectorRow label="Karte" hint={`0 bis ${mapMax}, leer = keine Karte`}>
          <MapField map={map} max={mapMax} onCommit={commitMap} />
        </InspectorRow>
        {map !== null && (
          <>
            <ImportPartsDialog
              map={map}
              open={partsDialog === "import"}
              onClose={() => setPartsDialog(null)}
              mapCount={counts.Map ?? 0}
              onImported={onMapImported}
              checks={checks}
              goTo={goTo}
            />
            <ExportPartDialog map={map} open={partsDialog === "export"} onClose={() => setPartsDialog(null)} />
          </>
        )}
        {header !== null && !tinker ? (
          <OwnerField matrix={index} header={header} headers={headers} identify={identify} goTo={goTo} onCommit={commitHeader} />
        ) : header !== null ? (
          <InspectorRow label={headerWord} hint={`0 bis ${headerMax}`}>
            <div className={styles.inline}>
              <div className={styles.grow}>
                <NumberField label={headerWord} value={header} max={headerMax} bits={16} onCommit={commitHeader} showBase={depth === "tuefteln"} />
              </div>
              {map !== null && header < headerCount && <Reference area="welt" label={headerWord} tech={id4(header)} onOpen={() => goTo({ kind: "header", id: header })} />}
            </div>
          </InspectorRow>
        ) : (
          // A room's cells have no area section: Bauen just leaves the field out, Tüfteln says where to switch the section on.
          tinker && <p className={styles.note}>Das Raster hat keinen Header-Abschnitt (in der Schau unter „Raster“ einschaltbar).</p>
        )}
        {/* Without an altitude section the height reads 0; the first height written makes the kernel add the section. */}
        <InspectorRow label="Höhe" hint="0 bis 255">
          <NumberField label="Höhe" value={altitude ?? 0} max={255} bits={8} onCommit={commitAltitude} showBase={depth === "tuefteln"} />
        </InspectorRow>
        {depth === "tuefteln" && <RawRowSection matrix={matrix} x={x} y={y} />}
        {depth === "tuefteln" && <TechnicalSection model={{ kind: "matrixCell", matrix: index, x, y }} source={{ kind: "matrixCell", map, header, altitude }} />}
        <p className={styles.note}>
          „Duplizieren“ (Menü „Aktionen“) trägt eine Kopie der Karte in diese Zelle ein und lässt sich nicht rückgängig machen: Die Kopie ist sofort als neue
          Karte am Ende des Archivs vorhanden.
        </p>
      </div>
    </div>
  );
}
