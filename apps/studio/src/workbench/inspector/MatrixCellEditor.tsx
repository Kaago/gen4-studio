import { useEffect, useId, useMemo, useRef, useState } from "react";
import { MATRIX_EMPTY, duplicateMap, getMap, setMatrixCell, worldAdopt } from "../../api";
import type { EdgePlace } from "../../api.world";
import type { AssetId, HeaderSummary, MemberCounts } from "../../api";
import type { ChecksApi } from "../../checks/useChecks";
import { useDelete } from "../../delete/DeleteContext";
import { useReset } from "../../original/OriginalContext";
import { useTextProject } from "../../textproject/TextProjectContext";
import { resetEntry } from "../../original/resetEntry";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Button, InspectorRow, Notice, NumberField, Picker, Reference, TextField, checkNumber } from "../../ui/primitives";
import type { MenuEntry, PickerItem } from "../../ui/primitives";
import type { HeaderIdentity } from "../headerIdentity";
import { cellLabel } from "../matrix";
import type { NavTarget } from "../navigation";
import { id4, termFor, termLabel, termShort } from "../terms";
import { useMatrix } from "../useMatrix";
import { tileParts } from "../../world/paint/ops";
import { usePaint } from "../paint/PaintContext";
import { useWorld } from "../WorkbenchContext";
import type { World } from "../worldModel";
import styles from "./Editor.module.css";
import { EdgeSetEditor, useEdgeSetEdits } from "./EdgeSetEditor";
import { PLACE_LABEL, withVariant } from "./edgeSets";
import { ExportPartDialog } from "./ExportPartDialog";
import { ImportPartsDialog } from "./ImportPartsDialog";
import { isDirty } from "./dirtyAssets";
import { InspectorHead } from "./InspectorHead";
import { LayerList } from "./LayerList";
import { MapBudgetSection } from "./MapBudgetSection";
import { RawRowSection } from "./RawRowSection";
import { TechnicalSection } from "./TechnicalSection";
import { TileEditor } from "./TileEditor";
import { TilePalette } from "./TilePalette";
import { ADOPT_NEEDS_TEXT_PROJECT, WorldSourceSection, adoptedText } from "./WorldSourceSection";

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
  /** Changes whenever a map's model, terrain or buildings may have changed (parts import, terrain, building edits, undo, redo): the budget reads again. */
  mapRevision?: number;
  onEdited: (index: number) => void | Promise<void>;
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
  /** Parts were imported into a map: pictures, scene and dirty count catch up. */
  onMapImported: (map: number) => void | Promise<void>;
  /** The checks of the project: the import dialog lists what they find at the map after an import. */
  checks: ChecksApi | null;
  /** The tool „Gelände malen“ is active: a world map's cell shows its palette, layers and tile; a binary map's the way to painting. */
  painting?: boolean;
  /** The project's game family (`ProjectInfo.family`: "Dp", "Pt", "Hgss"): the tile section shows the step sound for HeartGold/SoulSilver. */
  family?: string | null;
}

const NO_LAYERS: ReadonlySet<number> = new Set();

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
    return [{ id: LANDSCAPE, label: termLabel("landscape"), description: termShort("landscape") }, ...areas];
  }, [world, matrix]);
  const owner = header === LANDSCAPE ? undefined : headers.find((h) => h.id === header);
  return (
    <>
      <InspectorRow label="Gehört zu">
        <div className={styles.inline}>
          {header === LANDSCAPE ? (
            <span>{termLabel("landscape")}</span>
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

/**
 * Whether map `map` carries a world document: in a text project the map is read (`get_map`, its `world` flag) when it or `revision`
 * changes, only while `wanted` (from Bauen on); outside one every map is binary. Null until the first answer for this map (no binary
 * flash before a world), without a map and while not wanted. One read serves „Quelle“ and the note of a binary map while painting.
 */
function useMapIsWorld(map: number | null, revision: number, wanted: boolean, onError: (e: unknown) => void): boolean | null {
  const textProject = useTextProject().active;
  const [read, setRead] = useState<{ map: number; world: boolean } | null>(null);
  const report = useRef(onError);
  report.current = onError;
  const asked = wanted && textProject ? map : null;
  useEffect(() => {
    if (asked === null) return;
    let current = true;
    getMap(asked).then(
      (dto) => current && setRead({ map: asked, world: dto.world === true }),
      (e: unknown) => current && report.current(e),
    );
    return () => {
      current = false;
    };
  }, [asked, revision]);
  if (map === null || !wanted) return null;
  if (!textProject) return false;
  return read?.map === map ? read.world : null;
}

/**
 * The note of a binary map while „Gelände malen“ is active (Gutachten C §7.1): painting the ground needs the map as a world, so the note
 * offers „Karte als Quelle übernehmen“ (`world_adopt`, one undo step, as in „Quelle“); outside a text project the button is locked with
 * the reason. After the adoption `onChanged` lets the app catch up (the paint target follows the map's world flag); `onAdopted` gets the
 * notice; a refusal goes to `onError`.
 */
function PaintAdoptNote({
  map,
  textProject,
  onChanged,
  onAdopted,
  onError,
}: {
  map: number;
  textProject: boolean;
  onChanged: (map: number) => void | Promise<void>;
  onAdopted: (text: string) => void;
  onError: (e: unknown) => void;
}) {
  const [working, setWorking] = useState(false);
  const reasonId = useId();
  async function adopt() {
    if (working || !textProject) return;
    setWorking(true);
    try {
      const done = await worldAdopt(map);
      onAdopted(adoptedText(done));
      await onChanged(map);
    } catch (e) {
      onError(e);
    } finally {
      setWorking(false);
    }
  }
  return (
    <div data-paint-binary="">
      <Notice kind="info">Boden malen: Karte als Quelle übernehmen</Notice>
      <div className={styles.actions}>
        <Button
          kind="secondary"
          size="s"
          data-paint-adopt=""
          disabled={!textProject}
          aria-disabled={(textProject && working) || undefined}
          aria-describedby={textProject ? undefined : reasonId}
          onClick={() => void adopt()}
        >
          Karte als Quelle übernehmen
        </Button>
      </div>
      {!textProject && (
        <p id={reasonId} className={styles.note}>
          {ADOPT_NEEDS_TEXT_PROJECT}
        </p>
      )}
    </div>
  );
}

/** The family word of the tile section: HeartGold/SoulSilver has the step sound. */
const tileFamily = (family: string | null | undefined): "pt" | "hgss" => (family === "Hgss" ? "hgss" : "pt");

/** Editor of one matrix cell: map, header and altitude, each one undoable change. */
export function MatrixCellEditor({ matrix: index, x, y, context, depth }: { matrix: number; x: number; y: number; context: MatrixCellContext; depth: Depth }) {
  const { dirty, headers, identify, headerCount, goTo, counts, revision, mapRevision = 0, onEdited, onCountsChanged, onError, onMapImported, checks, painting = false, family = null } = context;
  const matrix = useMatrix(index, revision, onError);
  const textProject = useTextProject().active;
  // The cell's map, known before the matrix's own checks below: the map's world flag is read for „Quelle“ and the painting note.
  const cellMap = matrix && x < matrix.width && y < matrix.height ? (matrix.maps[y * matrix.width + x] ?? null) : null;
  const isWorld = useMapIsWorld(cellMap, revision + mapRevision, depthAtLeast(depth, "bauen"), onError);
  // The notice of the last adoption from the painting note, for the map it belongs to (shown while that map is a world).
  const [adopted, setAdopted] = useState<{ map: number; text: string } | null>(null);
  const paint = usePaint();
  const { world } = useWorld();
  const reset = useReset();
  const remove = useDelete();
  const [working, setWorking] = useState(false);
  const [partsDialog, setPartsDialog] = useState<"import" | "export" | null>(null);
  // The chosen place of the active edge set (section „Kantensätze“): while one is chosen, a palette click appends to it.
  const [edgePlace, setEdgePlace] = useState<EdgePlace | null>(null);
  // The edits of the edge sets, shared by the palette's append and the section: one at a time, each building on the last set written.
  const edgeEdits = useEdgeSetEdits(paint.tileset);

  const cell = `Zelle ${x}, ${y}`;
  // Bauen speaks of the world map and the area a cell belongs to; Tüfteln keeps the matrix ("Raster") and the header.
  const tinker = depthAtLeast(depth, "tuefteln");
  // Bauen names the map by its world map, area or room, never by its number (a cell outside or a matrix still loading).
  const matrixName = tinker ? `${termLabel("matrix")} ${id4(index)}` : matrixWords(world, index, headers, identify);
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

  // The map's actions: its parts in and out (only when the cell has a map), a copy of it in this cell, the map as a tombstone, and the map as
  // in the base ROM (maps have no list: this head is the one place to delete or reset one).
  const actions: MenuEntry[] = [
    ...(map === null
      ? []
      : [
          { label: "Teile importieren…", onSelect: () => setPartsDialog("import") },
          { label: "Teile exportieren…", onSelect: () => setPartsDialog("export") },
        ]),
    { label: "Duplizieren", disabled: map === null || working, onSelect: () => map !== null && void duplicate(map) },
    ...(map === null ? [] : [{ label: "Löschen…", onSelect: () => remove.open({ kind: "map", index: map }) }]),
    ...(map === null ? [] : ["separator" as const, resetEntry({ kind: "map", index: map }, reset.status, reset.open)]),
  ];
  // How the cell's map stands in the saved project against the base.
  const state = map === null ? null : remove.stateOf({ kind: "map", index: map });
  // While „Gelände malen“ paints this cell's world map: the world the paint session read last (palette, layers, the bake for the budget).
  const painted = map !== null && paint.target === map && paint.world?.map === map ? paint.world : null;
  const paintWorld = painting && depthAtLeast(depth, "bauen") ? (painted?.world ?? null) : null;
  // Choosing a tile goes back to the brush from the tools that paint no tile (Tiled: a click in the tileset takes the stamp).
  // The chosen tile of the palette (its four hex digits). After the tile section changed the tileset, `onMapImported` refetches the map:
  // the paint session reads its world and tileset again (palette and section show the new values), mesh and bake follow.
  const chosenTile = Number.parseInt(paint.state.tile.slice(0, 4), 16);
  const choosePlain = (tile: string) => paint.set((s) => (s.sub === "brush" || s.sub === "rect" || s.sub === "fill" ? { tile } : { tile, sub: "brush" }));
  // The active edge set of the paint state in this tileset, and the place chosen in it (none without a file: nothing can be written).
  const edgeTileset = edgeEdits.shown;
  const edgeSet = edgeTileset?.tileset.edge_sets?.find((s) => s.name === paint.state.edgeSet) ?? null;
  const placing = edgeSet !== null && edgeTileset?.stored === true ? edgePlace : null;
  // While a place is chosen, a palette click appends the clicked tile with the active turn and mirror (Z, X) to it: one `tileset_edit`
  // built on the last set written; the token becomes the active tile, the sub-tool stays (T or K, painting with the set). While another
  // edit of the sets runs the click does nothing (the palette's choice stays too).
  const chooseTile = (digits: string) => {
    if (placing === null || edgeSet === null || map === null) return choosePlain(digits);
    if (edgeEdits.running()) return;
    const turn = tileParts(paint.state.tile);
    const token = `${digits}${turn?.rotate ? `r${turn.rotate}` : ""}${turn?.flip ? "f" : ""}`;
    paint.set({ tile: token });
    const next = withVariant(edgeSet, placing, token);
    if (next === edgeSet) return;
    void (async () => {
      try {
        await edgeEdits.edit([], [{ put: next }]);
        await onMapImported(map);
      } catch (e) {
        onError(e);
      }
    })();
  };

  return (
    <div className={styles.panel}>
      <InspectorHead kind="map" title={title} ident={ident} state={state} unsaved={unsaved} actions={actions} depth={depth} />
      <div className={styles.body}>
        {/* While painting a binary map: the way to painting its ground, at the top. */}
        {map !== null && painting && isWorld === false && (
          <PaintAdoptNote map={map} textProject={textProject} onChanged={onMapImported} onAdopted={(text) => setAdopted({ map, text })} onError={onError} />
        )}
        {map !== null && painting && isWorld === true && adopted?.map === map && <Notice kind="success">{adopted.text}</Notice>}
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
              tinker={tinker}
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
                <NumberField label={headerWord} value={header} max={headerMax} bits={16} onCommit={commitHeader} showBase={depthAtLeast(depth, "tuefteln")} />
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
          <NumberField label="Höhe" value={altitude ?? 0} max={255} bits={8} onCommit={commitAltitude} showBase={depthAtLeast(depth, "tuefteln")} />
        </InspectorRow>
        {/* While painting a world map: its palette, layers and the chosen tile, before „Quelle“ and „Budget“. */}
        {map !== null && paintWorld && (
          <>
            {paint.tileset && (
              <TilePalette tileset={paint.tileset} tile={paint.state.tile} onTile={chooseTile} depth={depth} target={placing === null ? undefined : PLACE_LABEL[placing]} />
            )}
            <LayerList
              map={map}
              world={paintWorld}
              active={paint.state.layer}
              hidden={paint.hidden ?? NO_LAYERS}
              onActive={(layer) => paint.set({ layer })}
              onToggle={(layer) => paint.toggleHidden?.(layer)}
              onError={onError}
              onChanged={onMapImported}
              onRemoved={(layer) => paint.layerRemoved?.(layer)}
            />
            {paint.tileset && Number.isInteger(chosenTile) && (
              <TileEditor
                tileset={paint.tileset}
                tile={chosenTile}
                family={tileFamily(family)}
                depth={depth}
                onChanged={() => void onMapImported(map)}
                onError={onError}
              />
            )}
            {edgeTileset && (
              <EdgeSetEditor
                tileset={edgeTileset}
                busy={edgeEdits.busy}
                edit={edgeEdits.edit}
                active={paint.state.edgeSet}
                place={placing}
                tile={paint.state.tile}
                rise={paint.state.cliffRise}
                onRise={(cliffRise) => paint.set({ cliffRise })}
                onActive={(name) => {
                  setEdgePlace(null);
                  paint.set({ edgeSet: name });
                }}
                onPlace={setEdgePlace}
                onChanged={() => void onMapImported(map)}
                onError={onError}
                depth={depth}
              />
            )}
          </>
        )}
        {/* Where the cell's map comes from (binary or world) and the way to the other, from Bauen on. */}
        {map !== null && isWorld !== null && (
          <WorldSourceSection map={map} world={isWorld} textProject={textProject} revision={revision + mapRevision} onChanged={onMapImported} onError={onError} />
        )}
        {/* The game's limits for the cell's map, from Bauen on. */}
        {map !== null && depthAtLeast(depth, "bauen") && <MapBudgetSection map={map} revision={revision + mapRevision} depth={depth} onError={onError} bake={painted?.bake} />}
        {depthAtLeast(depth, "tuefteln") && <RawRowSection matrix={matrix} x={x} y={y} />}
        {depthAtLeast(depth, "tuefteln") && <TechnicalSection model={{ kind: "matrixCell", matrix: index, x, y }} source={{ kind: "matrixCell", map, header, altitude }} />}
        <p className={styles.note}>
          „Duplizieren“ (Menü „Aktionen“) trägt eine Kopie der Karte in diese Zelle ein und lässt sich nicht rückgängig machen: Die Kopie ist sofort als neue
          Karte am Ende des Archivs vorhanden.
        </p>
      </div>
    </div>
  );
}
