import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { tilesetEdit, tilesetSave } from "../../api";
import type { EdgePlace, EdgeSetJson, EdgeSetOp, TileChange, TilesetDto } from "../../api.world";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { t as translate } from "../../i18n";
import { Button, InspectorRow, NumberField, Section, Segmented, TextField, Tooltip } from "../../ui/primitives";
import type { SegmentedOption } from "../../ui/primitives";
import { tileImage } from "../../world/paint/paletteImages";
import { tileParts } from "../../world/paint/ops";
import { PLACE_GRID, PLACE_LABEL, applyEdgeOps, completeness, newSet, setTiles, withoutVariant } from "./edgeSets";
import { useSectionOpen } from "./foldState";
import { WallField } from "./TileEditor";
import { TileFace, useTilePictures } from "./TilePalette";
import styles from "./EdgeSetEditor.module.css";

/** The cliff's steps (rule „Klippenhöhe in der App“): 1 to 8, default 2. */
export const RISE_MIN = 1;
export const RISE_MAX = 8;

/** Which cell of the cliff is blocked, in the language that is shown when called. */
const blocks = (): SegmentedOption<"upper" | "lower">[] => [
  { value: "upper", label: translate("workbench:edgeSetEditor.upperCell") },
  { value: "lower", label: translate("workbench:edgeSetEditor.lowerCell") },
];

/** The grid in its three blocks: 3 × 3 (area, edges, outer corners), 2 × 2 (inner corners), the row of the diagonals. */
const GRID_BLOCKS: EdgePlace[][][] = [PLACE_GRID.slice(0, 3), PLACE_GRID.slice(3, 5), PLACE_GRID.slice(5)].map((rows) =>
  rows.map((row) => row.filter((p): p is EdgePlace => p !== null)),
);

/** „13 von 13 Plätzen · Diagonalen 0 von 2“. */
function countText(set: EdgeSetJson): string {
  const c = completeness(set);
  return translate("workbench:edgeSetEditor.count", { places: c.places, diagonals: c.diagonals });
}

/** „2 Kacheln“, „1 Kachel“, „keine Kachel“. */
const tilesWord = (n: number) => (n === 0 ? translate("workbench:edgeSetEditor.noTile") : translate("workbench:edgeSetEditor.tiles", { count: n }));

/** A free proposal for a new set's name: „Satz <n>“ from the number of sets on. */
function proposedName(sets: readonly EdgeSetJson[]): string {
  const name = (n: number) => translate("workbench:edgeSetEditor.proposedName", { n });
  let n = sets.length + 1;
  while (sets.some((s) => s.name === name(n))) n += 1;
  return name(n);
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
    if (running.current) throw new Error(translate("workbench:edgeSetEditor.stillRunning"));
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
 * place as chips with „Entfernen“, „Außen“, „Blockiert an der Klippe“, „Wand“ (the wall of every tile of the set, the outside too) and „Satz
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
  const { t } = useTranslation("workbench");
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
    if (wanted === "") return setNameError(t("edgeSetEditor.nameMissing"));
    if (sets.some((s) => s.name === wanted)) return setNameError(t("edgeSetEditor.nameTaken", { name: wanted }));
    void run(
      () => edit([], [{ put: newSet(wanted, tile) }]),
      () => onActive(wanted),
    );
  }

  /** The face of a token: the picture of its tile turned like the mesh, else its role's colour. */
  function face(token: string) {
    const parts = tileParts(token);
    const found = parts ? tiles[Number.parseInt(parts.digits, 16)] : undefined;
    if (!parts || !found) return <span className={styles.unknown} aria-hidden="true" />;
    return <TileFace bitmap={pictures.get(found.texture) ?? null} role={found.role} rotate={parts.rotate} flip={parts.flip} />;
  }
  const tileWord = (token: string) => (tinker ? token : t("edgeSetEditor.tile", { digits: tileParts(token)?.digits ?? token }));

  // The wall all tiles of the set share, the places and the outside (empty when they differ or have none).
  const wallTiles = set ? setTiles(set) : [];
  const walls = new Set(wallTiles.map((i) => tiles[i]?.wall));
  const commonWall = walls.size === 1 ? [...walls][0] : undefined;
  const variants = set && place ? (set.places[place] ?? []) : [];

  return (
    <div data-edge-sets="">
      <Section title={t("edgeSetEditor.title")} open={open} onToggle={toggle}>
        <div className={styles.body}>
          {!tileset.stored && (
            <div className={styles.save}>
              <Button kind="secondary" size="s" data-tileset-save="" aria-disabled={working || undefined} onClick={() => void run(save)}>
                {t("edgeSetEditor.saveTileset")}
              </Button>
              <p className={styles.note}>{t("edgeSetEditor.saveNote", { package: String(tileset.package).padStart(4, "0") })}</p>
            </div>
          )}
          {sets.length === 0 ? (
            <p className={styles.note}>{t("edgeSetEditor.none")}</p>
          ) : (
            <ul className={styles.rows}>
              {sets.map((s) => (
                <li key={s.name} className={styles.row} data-edge-set={s.name}>
                  <input type="radio" className={styles.radio} name={group} aria-label={t("edgeSetEditor.active")} checked={s.name === active} onChange={() => onActive(s.name)} />
                  <span className={styles.name}>{s.name}</span>
                  <span className={styles.count}>{countText(s)}</span>
                </li>
              ))}
            </ul>
          )}
          <div className={styles.add}>
            <div className={styles.grow}>
              <TextField
                label={t("edgeSetEditor.newName")}
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
              {t("edgeSetEditor.add")}
            </Button>
          </div>
          <div data-cliff-rise="">
            <InspectorRow label={t("edgeSetEditor.rise")} hint={t("edgeSetEditor.riseHint")}>
              <NumberField
                label={t("edgeSetEditor.rise")}
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
                <p className={styles.note}>{t("edgeSetEditor.choosePlace")}</p>
              ) : (
                <div className={styles.variants} data-edge-variants={place}>
                  <span className={styles.variantsTitle}>{PLACE_LABEL[place]}</span>
                  {variants.length === 0 && <p className={styles.note}>{t("edgeSetEditor.noVariant")}</p>}
                  <div className={styles.chips}>
                    {variants.map((token) => (
                      <span key={token} className={styles.chip} data-edge-variant={token}>
                        <span className={styles.chipFace}>{face(token)}</span>
                        {tileWord(token)}
                        <Button kind="quiet" size="s" aria-label={t("edgeSetEditor.removeTile", { tile: tileWord(token) })} disabled={locked} onClick={() => put(withoutVariant(set, place, token))}>
                          {t("edgeSetEditor.remove")}
                        </Button>
                      </span>
                    ))}
                  </div>
                </div>
              )}
              <InspectorRow label={t("edgeSetEditor.outside")}>
                <div className={styles.inline}>
                  <span className={styles.chipFace}>{face(set.outside)}</span>
                  <span className={styles.name}>{tileWord(set.outside)}</span>
                  <Button kind="quiet" size="s" data-edge-outside="" disabled={locked || tile === set.outside} onClick={() => put({ ...set, outside: tile })}>
                    {t("edgeSetEditor.useActiveTile")}
                  </Button>
                </div>
              </InspectorRow>
              <div data-edge-block="">
                <InspectorRow label={t("edgeSetEditor.blocked")}>
                  <Segmented
                    ariaLabel={t("edgeSetEditor.blocked")}
                    size="s"
                    options={blocks().map((o) => ({ ...o, disabled: locked }))}
                    value={set.block ?? "upper"}
                    onChange={(block) => {
                      const { block: _old, ...rest } = set;
                      put(block === "lower" ? { ...rest, block } : rest);
                    }}
                  />
                </InspectorRow>
              </div>
              <div data-edge-wall="">
                <InspectorRow label={t("edgeSetEditor.wall")} hint={t("edgeSetEditor.wallHint")}>
                  <WallField
                    wall={commonWall}
                    last={Math.max(0, tiles.length - 1)}
                    disabled={locked || wallTiles.length === 0}
                    onCommit={(wall) => put(set, wallTiles.map((index) => ({ tile: index, wall })))}
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
                  {t("edgeSetEditor.removeSet")}
                </Button>
              </div>
            </div>
          )}
        </div>
      </Section>
    </div>
  );
}
