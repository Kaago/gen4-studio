import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import { tilesetEdit, tilesetSave } from "../../api";
import type { EdgePlace, EdgeSetJson, EdgeSetOp, TileChange, TilesetDto } from "../../api.world";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Button, InspectorRow, NumberField, Section, Segmented, TextField, Tooltip } from "../../ui/primitives";
import type { SegmentedOption } from "../../ui/primitives";
import { tileImage } from "../../world/paint/paletteImages";
import { tileParts } from "../../world/paint/ops";
import { PLACE_GRID, PLACE_LABEL, applyEdgeOps, completeness, newSet, placeTiles, withoutVariant } from "./edgeSets";
import { useSectionOpen } from "./foldState";
import { WallField } from "./TileEditor";
import { TileFace, useTilePictures } from "./TilePalette";
import styles from "./EdgeSetEditor.module.css";

/** The cliff's steps (rule „Klippenhöhe in der App“): 1 to 8, default 2. */
export const RISE_MIN = 1;
export const RISE_MAX = 8;

const BLOCKS: readonly SegmentedOption<"upper" | "lower">[] = [
  { value: "upper", label: "Obere Zelle" },
  { value: "lower", label: "Untere Zelle" },
];

/** The grid in its three blocks: 3 × 3 (area, edges, outer corners), 2 × 2 (inner corners), the row of the diagonals. */
const GRID_BLOCKS: EdgePlace[][][] = [PLACE_GRID.slice(0, 3), PLACE_GRID.slice(3, 5), PLACE_GRID.slice(5)].map((rows) =>
  rows.map((row) => row.filter((p): p is EdgePlace => p !== null)),
);

/** „13 von 13 Plätzen · Diagonalen 0 von 2“. */
function countText(set: EdgeSetJson): string {
  const c = completeness(set);
  return `${c.places} von 13 Plätzen · Diagonalen ${c.diagonals} von 2`;
}

/** „2 Kacheln“, „1 Kachel“, „keine Kachel“. */
const tilesWord = (n: number) => (n === 0 ? "keine Kachel" : n === 1 ? "1 Kachel" : `${n} Kacheln`);

/** A free proposal for a new set's name: „Satz <n>“ from the number of sets on. */
function proposedName(sets: readonly EdgeSetJson[]): string {
  let n = sets.length + 1;
  while (sets.some((s) => s.name === `Satz ${n}`)) n += 1;
  return `Satz ${n}`;
}

/**
 * One funnel for the edge set edits of a world map's tileset (the palette's append and the section's own changes): one edit at a time
 * (`busy`; `edit` refuses a second one while the first runs), and until the tileset is read again after an edit, `shown` is the read
 * tileset with the edits written since applied, so the next change builds on the last set written, never on the stale one (a quick
 * second click would otherwise put the set without the first tile). A newly read tileset (refetch, undo, another package) drops them.
 */
export function useEdgeSetEdits(tileset: TilesetDto | null | undefined): {
  shown: TilesetDto | null;
  busy: boolean;
  /** Whether an edit runs right now (also before `busy` reaches the next render). */
  running: () => boolean;
  edit: (changes: TileChange[], sets: EdgeSetOp[]) => Promise<unknown>;
} {
  const [written, setWritten] = useState<{ base: TilesetDto; ops: EdgeSetOp[] } | null>(null);
  const [busy, setBusy] = useState(false);
  const running = useRef(false);
  const current = useRef(tileset);
  current.current = tileset;
  const shown = useMemo(() => (tileset && written?.base === tileset ? applyEdgeOps(tileset, written.ops) : (tileset ?? null)), [tileset, written]);
  const edit = useCallback(async (changes: TileChange[], sets: EdgeSetOp[]) => {
    const base = current.current;
    if (!base) return;
    if (running.current) throw new Error("Die vorige Änderung am Kantensatz läuft noch.");
    running.current = true;
    setBusy(true);
    try {
      await tilesetEdit(base.package, changes, sets);
      const now = current.current;
      if (now) setWritten((w) => (w?.base === now ? { base: now, ops: [...w.ops, ...sets] } : { base: now, ops: sets }));
    } finally {
      running.current = false;
      setBusy(false);
    }
  }, []);
  const isRunning = useCallback(() => running.current, []);
  return { shown, busy, running: isRunning, edit };
}

/**
 * The section „Kantensätze“ of a world map while painting (rule „Kantensatz-Editor: Ort und Bedienung“): the sets of the map's tileset
 * with the radio „Aktiv“ (the set T and K paint with, `onActive`) and their completeness, „Neuer Kantensatz“ (a name and the active
 * palette tile as its outside), the cliff's steps (state only) and the editor of the active set: the 13 + 2 places in their blocks, one
 * chosen by a click (`onPlace`; a palette click then appends the active token there, see `MatrixCellEditor`), the variants of the chosen
 * place as chips with „Entfernen“, „Außen“, „Blockiert an der Klippe“, „Wand“ (the wall of every tile of the places) and „Satz
 * entfernen“. Each confirmed change is one `tileset_edit` (one file step) with `sets: [{put}]` or `[{remove}]`, then `onChanged`; a
 * refusal (a token twice in the set, …) goes to `onError`. A derived tileset (`stored: false`) locks the changes and offers „Tileset im
 * Projekt anlegen“ as the section „Kachel“ does. Tüfteln writes the tokens under each place.
 */
export function EdgeSetEditor({
  tileset,
  active,
  place,
  tile,
  onActive,
  onPlace,
  onChanged,
  onError,
  depth,
  rise = 2,
  onRise = () => {},
  busy = false,
  edit = (changes, sets) => tilesetEdit(tileset.package, changes, sets),
  save = () => tilesetSave(tileset.package),
  image = tileImage,
}: {
  tileset: TilesetDto;
  active: string | null;
  place: EdgePlace | null;
  /** The active palette token (with turn and mirror): the outside of a new set and of „Außen“. */
  tile: string;
  onActive(name: string | null): void;
  onPlace(place: EdgePlace | null): void;
  onChanged(): void;
  onError(e: unknown): void;
  depth: Depth;
  /** The cliff's steps of the paint state and their change (1 to 8). */
  rise?: number;
  onRise?: (rise: number) => void;
  /** Another edit of the sets runs (the palette's append): the changes wait. */
  busy?: boolean;
  /** Where the changes go (the gallery writes nothing). */
  edit?: (changes: TileChange[], sets: EdgeSetOp[]) => Promise<unknown>;
  save?: () => Promise<unknown>;
  /** Where the pictures come from (the gallery has none). */
  image?: (pkg: number, texture: string) => Promise<ImageBitmap | null>;
}): JSX.Element {
  const [open, toggle] = useSectionOpen("matrixCell:edgeSets", true);
  const tinker = depthAtLeast(depth, "tuefteln");
  const group = useId();
  const tiles = tileset.tileset.tiles;
  const sets = useMemo(() => tileset.tileset.edge_sets ?? [], [tileset]);
  const set = sets.find((s) => s.name === active) ?? null;
  const pictures = useTilePictures(tileset.package, tiles, image);
  const [working, setWorking] = useState(false);
  const locked = !tileset.stored || working || busy;
  const report = useRef(onError);
  report.current = onError;

  const proposal = proposedName(sets);
  const [name, setName] = useState(proposal);
  const [nameError, setNameError] = useState<string | undefined>();
  // Another list of sets proposes the next free name again.
  useEffect(() => {
    setName(proposal);
    setNameError(undefined);
  }, [proposal]);

  async function run(op: () => Promise<unknown>, after?: () => void) {
    if (working || busy) return;
    setWorking(true);
    try {
      await op();
      after?.();
      onChanged();
    } catch (e) {
      report.current(e);
    } finally {
      setWorking(false);
    }
  }
  const put = (next: EdgeSetJson, changes: TileChange[] = []) => void run(() => edit(changes, [{ put: next }]));

  function create() {
    const wanted = name.trim();
    if (wanted === "") return setNameError("Ein Kantensatz braucht einen Namen.");
    if (sets.some((s) => s.name === wanted)) return setNameError(`Einen Kantensatz „${wanted}“ gibt es schon.`);
    void run(
      () => edit([], [{ put: newSet(wanted, tile) }]),
      () => onActive(wanted),
    );
  }

  /** The face of a token: the picture of its tile turned like the mesh, else its role's colour. */
  function face(token: string) {
    const parts = tileParts(token);
    const t = parts ? tiles[Number.parseInt(parts.digits, 16)] : undefined;
    if (!parts || !t) return <span className={styles.unknown} aria-hidden="true" />;
    return <TileFace bitmap={pictures.get(t.texture) ?? null} role={t.role} rotate={parts.rotate} flip={parts.flip} />;
  }
  const tileWord = (token: string) => (tinker ? token : `Kachel ${tileParts(token)?.digits ?? token}`);

  // The wall all tiles of the places share (empty when they differ or have none).
  const wallTiles = set ? placeTiles(set) : [];
  const walls = new Set(wallTiles.map((i) => tiles[i]?.wall));
  const commonWall = walls.size === 1 ? [...walls][0] : undefined;
  const variants = set && place ? (set.places[place] ?? []) : [];

  return (
    <div data-edge-sets="">
      <Section title="Kantensätze" open={open} onToggle={toggle}>
        <div className={styles.body}>
          {!tileset.stored && (
            <div className={styles.save}>
              <Button kind="secondary" size="s" data-tileset-save="" aria-disabled={working || undefined} onClick={() => void run(save)}>
                Tileset im Projekt anlegen
              </Button>
              <p className={styles.note}>Kantensätze stehen in data/tilesets/{String(tileset.package).padStart(4, "0")}.json; erst dann lassen sie sich anlegen und ändern.</p>
            </div>
          )}
          {sets.length === 0 ? (
            <p className={styles.note}>Noch kein Kantensatz. Ein neuer nimmt die gewählte Palettenkachel als Außenfläche.</p>
          ) : (
            <ul className={styles.rows}>
              {sets.map((s) => (
                <li key={s.name} className={styles.row} data-edge-set={s.name}>
                  <input type="radio" className={styles.radio} name={group} aria-label="Aktiv" checked={s.name === active} onChange={() => onActive(s.name)} />
                  <span className={styles.name}>{s.name}</span>
                  <span className={styles.count}>{countText(s)}</span>
                </li>
              ))}
            </ul>
          )}
          <div className={styles.add}>
            <div className={styles.grow}>
              <TextField
                label="Name des neuen Kantensatzes"
                value={name}
                error={nameError}
                disabled={locked}
                onChange={() => setNameError(undefined)}
                onCommit={(text) => {
                  setNameError(undefined);
                  setName(text);
                }}
              />
            </div>
            <Button kind="secondary" size="s" disabled={locked} onClick={create}>
              Neuer Kantensatz
            </Button>
          </div>
          <div data-cliff-rise="">
            <InspectorRow label="Stufen der Klippe" hint="1 bis 8, je Stufe 8 E">
              <NumberField
                label="Stufen der Klippe"
                value={rise}
                min={RISE_MIN}
                max={RISE_MAX}
                bits={8}
                onCommit={(n) => n >= RISE_MIN && n <= RISE_MAX && n !== rise && onRise(n)}
              />
            </InspectorRow>
          </div>
          {set && (
            <div className={styles.editor} data-edge-editor="">
              <div className={styles.grid}>
                {GRID_BLOCKS.map((rows, b) => (
                  <div key={b} className={styles.block} data-columns={rows[0].length}>
                    {rows.flat().map((p) => {
                      const tokens = set.places[p] ?? [];
                      const first = tokens[0];
                      return (
                        <div key={p} className={styles.cell}>
                          <Tooltip content={PLACE_LABEL[p]}>
                            <button
                              type="button"
                              className={styles.place}
                              data-edge-place={p}
                              data-edge-empty={first === undefined ? "" : undefined}
                              aria-pressed={place === p}
                              aria-label={`${PLACE_LABEL[p]}, ${tilesWord(tokens.length)}`}
                              onClick={() => onPlace(place === p ? null : p)}
                            >
                              {first !== undefined && face(first)}
                              {tokens.length > 1 && (
                                <span className={styles.badge} data-edge-count="" aria-hidden="true">
                                  {tokens.length}
                                </span>
                              )}
                            </button>
                          </Tooltip>
                          {tinker && tokens.length > 0 && (
                            <span className={styles.tokens} data-edge-tokens="">
                              {tokens.join(" ")}
                            </span>
                          )}
                        </div>
                      );
                    })}
                  </div>
                ))}
              </div>
              {place === null ? (
                <p className={styles.note}>Einen Platz wählen, dann eine Kachel in der Palette: sie kommt mit ihrer Drehung (Z, X) in den Platz.</p>
              ) : (
                <div className={styles.variants} data-edge-variants={place}>
                  <span className={styles.variantsTitle}>{PLACE_LABEL[place]}</span>
                  {variants.length === 0 && <p className={styles.note}>Noch keine Kachel: ein Klick in der Palette hängt eine an.</p>}
                  <div className={styles.chips}>
                    {variants.map((token) => (
                      <span key={token} className={styles.chip} data-edge-variant={token}>
                        <span className={styles.chipFace}>{face(token)}</span>
                        {tileWord(token)}
                        <Button kind="quiet" size="s" aria-label={`${tileWord(token)} entfernen`} disabled={locked} onClick={() => put(withoutVariant(set, place, token))}>
                          Entfernen
                        </Button>
                      </span>
                    ))}
                  </div>
                </div>
              )}
              <InspectorRow label="Außen">
                <div className={styles.inline}>
                  <span className={styles.chipFace}>{face(set.outside)}</span>
                  <span className={styles.name}>{tileWord(set.outside)}</span>
                  <Button kind="quiet" size="s" data-edge-outside="" disabled={locked || tile === set.outside} onClick={() => put({ ...set, outside: tile })}>
                    Aktive Kachel übernehmen
                  </Button>
                </div>
              </InspectorRow>
              <div data-edge-block="">
                <InspectorRow label="Blockiert an der Klippe">
                  <Segmented
                    ariaLabel="Blockiert an der Klippe"
                    size="s"
                    options={BLOCKS.map((o) => ({ ...o, disabled: locked }))}
                    value={set.block ?? "upper"}
                    onChange={(block) => {
                      const { block: _old, ...rest } = set;
                      put(block === "lower" ? { ...rest, block } : rest);
                    }}
                  />
                </InspectorRow>
              </div>
              <div data-edge-wall="">
                <InspectorRow label="Wand" hint="Für jede Kachel der Plätze; Kachelnummer, leer = keine">
                  <WallField
                    wall={commonWall}
                    last={Math.max(0, tiles.length - 1)}
                    disabled={locked || wallTiles.length === 0}
                    onCommit={(wall) => put(set, wallTiles.map((t) => ({ tile: t, wall })))}
                  />
                </InspectorRow>
              </div>
              <div className={styles.actions}>
                <Button
                  kind="quiet"
                  size="s"
                  disabled={locked}
                  onClick={() =>
                    void run(
                      () => edit([], [{ remove: { name: set.name } }]),
                      () => {
                        onPlace(null);
                        onActive(null);
                      },
                    )
                  }
                >
                  Satz entfernen
                </Button>
              </div>
            </div>
          )}
        </div>
      </Section>
    </div>
  );
}
