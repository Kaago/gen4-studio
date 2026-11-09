import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { JSX } from "react";
import type { TileJson, TileRole, TilesetDto } from "../../api.world";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Section, Tooltip } from "../../ui/primitives";
import { onTileImagesReset, tileImage, tileImagesGeneration } from "../../world/paint/paletteImages";
import { useSectionOpen } from "./foldState";
import styles from "./TilePalette.module.css";

/** The groups of the palette in their order, with their headings. */
const GROUPS: { role: TileRole; label: string }[] = [
  { role: "ground", label: "Boden" },
  { role: "grass", label: "Gras" },
  { role: "water", label: "Wasser" },
  { role: "wall", label: "Wand" },
  { role: "ramp", label: "Rampe" },
  { role: "piece", label: "Teilmodell" },
];
const ROLE_WORD = Object.fromEntries(GROUPS.map((g) => [g.role, g.label])) as Record<TileRole, string>;

/** The four hex digits of tile `index`, as the tokens of the world document write them (`0041`). */
const digitsOf = (index: number) => index.toString(16).padStart(4, "0");
/** A byte in hex for Tüfteln („0x02“). */
const hex2 = (n: number) => `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;

/** The digits, turn and mirror of a tile token (`0041r1f`); null for `.` or a token outside the grammar. */
function tokenParts(token: string): { digits: string; rotate: number; flip: boolean } | null {
  const m = /^([0-9a-f]{4})(?:r([1-3]))?(f)?$/.exec(token.toLowerCase());
  return m ? { digits: m[1], rotate: m[2] ? Number(m[2]) : 0, flip: m[3] !== undefined } : null;
}

/** The small mark of a turned or mirrored tile: „90°“, „⇆“ or both; null for a plain one. */
function markOf(rotate: number, flip: boolean): string | null {
  const parts = [rotate > 0 ? `${rotate * 90}°` : null, flip ? "⇆" : null].filter((p) => p !== null);
  return parts.length > 0 ? parts.join(" ") : null;
}

/** The tooltip of a tile: „Kachel 0041 · gras_a · 32 × 32 Texel · Gras“, Tüfteln adds „kind 0x02, Kollision 0x00“. */
function tooltipOf(digits: string, tile: TileJson, tinker: boolean): string {
  const parts = [`Kachel ${digits}`, tile.texture, `${tile.size[0]} × ${tile.size[1]} Texel`, ROLE_WORD[tile.role] ?? tile.role];
  if (tinker) parts.push(`kind ${hex2(tile.kind)}, Kollision ${hex2(tile.collision)}`);
  return parts.join(" · ");
}

/** A tile's picture drawn into a canvas, texel for texel; turned clockwise `rotate` quarter turns and mirrored after the turn like the mesh. */
function TilePicture({ bitmap, rotate = 0, flip = false }: { bitmap: ImageBitmap; rotate?: number; flip?: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, el.width, el.height);
    ctx.drawImage(bitmap, 0, 0, el.width, el.height);
  }, [bitmap]);
  return (
    <canvas
      ref={canvas}
      className={styles.picture}
      width={32}
      height={32}
      data-palette-turn={rotate > 0 ? rotate : undefined}
      data-palette-flip={flip ? "" : undefined}
      aria-hidden="true"
    />
  );
}

/**
 * The pictures of package `pkg` by texture name (`image`, null: none); a package switch starts empty. A reset of the store (another
 * project, the package's files written) asks for them again; until they come, the old pictures stay (never closed by the store).
 * The palette and the edge set editor share it.
 */
export function useTilePictures(
  pkg: number,
  tiles: readonly TileJson[],
  image: (pkg: number, texture: string) => Promise<ImageBitmap | null>,
): ReadonlyMap<string, ImageBitmap | null> {
  const generation = useSyncExternalStore(onTileImagesReset, tileImagesGeneration);
  const [pictures, setPictures] = useState<{ pkg: number; byTexture: ReadonlyMap<string, ImageBitmap | null> }>({ pkg, byTexture: new Map() });
  useEffect(() => {
    let live = true;
    setPictures((shown) => (shown.pkg === pkg ? shown : { pkg, byTexture: new Map() }));
    for (const texture of new Set(tiles.map((t) => t.texture))) {
      void image(pkg, texture).then((bitmap) => {
        if (!live) return;
        setPictures((shown) => (shown.pkg === pkg ? { pkg, byTexture: new Map(shown.byTexture).set(texture, bitmap) } : shown));
      });
    }
    return () => {
      live = false;
    };
  }, [pkg, tiles, image, generation]);
  return pictures.pkg === pkg ? pictures.byTexture : NO_PICTURES;
}

const NO_PICTURES: ReadonlyMap<string, ImageBitmap | null> = new Map();

/** A tile's face: its picture (turned and mirrored like the mesh), else the colour of its role. */
export function TileFace({ bitmap, role, rotate = 0, flip = false }: { bitmap: ImageBitmap | null; role: TileRole; rotate?: number; flip?: boolean }) {
  return bitmap ? <TilePicture bitmap={bitmap} rotate={rotate} flip={flip} /> : <span className={styles.swatch} data-palette-color={role} aria-hidden="true" />;
}

/**
 * The palette of a world map while painting (section „Palette“): the tiles of its package (`tileset_get`) grouped by role in the order
 * Boden, Gras, Wasser, Wand, Rampe, Teilmodell (empty groups left out), one 32 × 32 key each with the whole texture of the tile
 * (`tileImage`), or the colour of its role without one. The chosen tile is pressed and shows its turn and mirror as a small mark and in its picture (turned like the mesh); a click
 * chooses a tile plain. Tüfteln writes the number under each tile and adds kind and collision to the tooltip. While a place of an edge set
 * is chosen (`target`, its name), a note says that a click goes to that place (the owner appends the tile there).
 */
export function TilePalette({
  tileset,
  tile,
  onTile,
  depth,
  image = tileImage,
  target,
}: {
  tileset: TilesetDto;
  tile: string;
  onTile: (token: string) => void;
  depth: Depth;
  /** Where the pictures come from (the gallery has none). */
  image?: (pkg: number, texture: string) => Promise<ImageBitmap | null>;
  /** The name of the chosen place of an edge set („Kante N“): a click appends the tile there. */
  target?: string;
}): JSX.Element {
  const [open, toggle] = useSectionOpen("matrixCell:palette", true);
  const tinker = depthAtLeast(depth, "tuefteln");
  const pkg = tileset.package;
  const tiles = tileset.tileset.tiles;
  const byTexture = useTilePictures(pkg, tiles, image);
  const groups = useMemo(
    () =>
      GROUPS.map((g) => ({ ...g, tiles: tiles.flatMap((t, index) => (t.role === g.role ? [{ tile: t, digits: digitsOf(index) }] : [])) })).filter(
        (g) => g.tiles.length > 0,
      ),
    [tiles],
  );
  const chosen = tokenParts(tile);
  return (
    <div data-paint-palette="">
      <Section title="Palette" open={open} onToggle={toggle}>
        <div className={styles.body}>
          {target !== undefined && (
            <p className={styles.target} data-palette-target="">
              Ein Klick hängt die Kachel (mit Drehung) an den Platz „{target}“.
            </p>
          )}
          {groups.map((g) => (
            <div key={g.role} className={styles.group} data-palette-role={g.role}>
              <h4 className={styles.groupTitle}>{g.label}</h4>
              <div className={styles.tiles}>
                {g.tiles.map(({ tile: t, digits }) => {
                  const pressed = chosen?.digits === digits;
                  const mark = pressed && chosen ? markOf(chosen.rotate, chosen.flip) : null;
                  const bitmap = byTexture.get(t.texture) ?? null;
                  return (
                    <div key={digits} className={styles.cell}>
                      <Tooltip content={tooltipOf(digits, t, tinker)}>
                        <button
                          type="button"
                          className={styles.tile}
                          data-palette-tile={digits}
                          aria-label={`Kachel ${digits}, ${t.texture}`}
                          aria-pressed={pressed}
                          onClick={() => onTile(digits)}
                        >
                          <TileFace bitmap={bitmap} role={t.role} rotate={pressed ? chosen?.rotate : 0} flip={pressed && chosen?.flip} />
                          {mark && (
                            <span className={styles.mark} data-palette-mark="" aria-hidden="true">
                              {mark}
                            </span>
                          )}
                        </button>
                      </Tooltip>
                      {tinker && (
                        <span className={styles.number} data-palette-number="">
                          {digits}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </Section>
    </div>
  );
}
