import { szene } from "../../tokens/tokens";

/**
 * Example preview pictures for the gallery (phase 4.0 design, §3.6): drawn here on a canvas of 256 × 192 from the scene tokens
 * (terrain, heights, events), tile by tile like a map seen from above. No game graphics, no ROM data.
 */

const WIDTH = 256;
const HEIGHT = 192;
/** One map tile in DS pixels: 16 × 12 tiles fill the screen. */
const TILE = 16;
const PREFIX = "data:image/png;base64,";

type Paint = (ctx: CanvasRenderingContext2D) => void;

/** A scene colour (a number for three.js) as CSS. */
function css(color: number): string {
  return `rgb(${(color >> 16) & 255} ${(color >> 8) & 255} ${color & 255})`;
}

function fill(ctx: CanvasRenderingContext2D, color: number, x: number, y: number, w: number, h: number) {
  ctx.fillStyle = css(color);
  ctx.fillRect(x, y, w, h);
}

/** Tiles `x0..x1` × `y0..y1` (inclusive) in one colour. */
function tiles(ctx: CanvasRenderingContext2D, color: number, x0: number, y0: number, x1: number, y1: number) {
  fill(ctx, color, x0 * TILE, y0 * TILE, (x1 - x0 + 1) * TILE, (y1 - y0 + 1) * TILE);
}

/** An event marker on a tile, in whole pixels (a round figure is a cross of rectangles, a warp a stepped diamond). */
function marker(ctx: CanvasRenderingContext2D, kind: "figur" | "warp" | "schild" | "gegenstand", tx: number, ty: number) {
  const x = tx * TILE;
  const y = ty * TILE;
  const color = szene.ereignis[kind];
  const ring = szene.auswahl.kern;
  if (kind === "figur") {
    fill(ctx, ring, x + 4, y + 2, 8, 12);
    fill(ctx, ring, x + 2, y + 4, 12, 8);
    fill(ctx, color, x + 5, y + 3, 6, 10);
    fill(ctx, color, x + 3, y + 5, 10, 6);
  } else if (kind === "warp") {
    for (let i = 0; i < 6; i++) fill(ctx, color, x + 7 - i, y + 2 + i, 2 + 2 * i, 1);
    for (let i = 0; i < 6; i++) fill(ctx, color, x + 2 + i, y + 8 + i, 12 - 2 * i, 1);
  } else if (kind === "schild") {
    fill(ctx, ring, x + 2, y + 3, 12, 8);
    fill(ctx, color, x + 3, y + 4, 10, 6);
    fill(ctx, color, x + 7, y + 11, 2, 3);
  } else {
    fill(ctx, ring, x + 4, y + 4, 8, 8);
    fill(ctx, color, x + 5, y + 5, 6, 6);
  }
}

/** Fine lines of the tile grid over the whole picture. */
function grid(ctx: CanvasRenderingContext2D) {
  ctx.globalAlpha = 0.18;
  for (let x = TILE; x < WIDTH; x += TILE) fill(ctx, szene.ds.grund, x, 0, 1, HEIGHT);
  for (let y = TILE; y < HEIGHT; y += TILE) fill(ctx, szene.ds.grund, 0, y, WIDTH, 1);
  ctx.globalAlpha = 1;
}

/** An own picture: a route by a lake, in the terrain colours, with a figure, a sign and an item on the path. */
const ROUTE: Paint = (ctx) => {
  const g = szene.gelaende;
  tiles(ctx, g["hohes-gras"], 0, 0, 15, 11);
  tiles(ctx, g.wasser, 9, 0, 15, 4);
  tiles(ctx, g.wasser, 11, 5, 15, 6);
  // Waves: light rows on the water.
  ctx.globalAlpha = 0.5;
  for (const [tx, ty] of [[10, 1], [13, 2], [11, 4], [14, 5], [12, 6]] as const) fill(ctx, g.eis, tx * TILE + 3, ty * TILE + 7, 10, 2);
  ctx.globalAlpha = 1;
  // The path from the lower left up to the shore, a ledge across it, rocks beside it.
  tiles(ctx, g.unbekannt, 1, 9, 6, 10);
  tiles(ctx, g.unbekannt, 5, 3, 6, 9);
  tiles(ctx, g.unbekannt, 6, 3, 8, 4);
  tiles(ctx, g.vorsprung, 2, 7, 4, 7);
  tiles(ctx, g.blockiert, 1, 2, 2, 3);
  tiles(ctx, g.blockiert, 10, 9, 11, 10);
  grid(ctx);
  marker(ctx, "figur", 5, 6);
  marker(ctx, "schild", 7, 3);
  marker(ctx, "gegenstand", 2, 10);
};

/** A standard picture: the start town from above in the height colours, houses with doors, a pond and the people in it. */
const TOWN: Paint = (ctx) => {
  const h = szene.hoehe;
  tiles(ctx, h["2"], 0, 0, 15, 11);
  tiles(ctx, h["3"], 1, 1, 14, 10);
  // Houses: a roof two heights up, the wall one below it, a door (warp) in the wall.
  for (const [x, y, w] of [[2, 2, 4], [10, 2, 4], [3, 7, 3]] as const) {
    tiles(ctx, h["6"], x, y, x + w - 1, y);
    tiles(ctx, h["5"], x, y + 1, x + w - 1, y + 1);
    tiles(ctx, h["4"], x, y + 2, x + w - 1, y + 2);
    marker(ctx, "warp", x + 1, y + 2);
  }
  tiles(ctx, szene.gelaende.wasser, 9, 7, 12, 9);
  tiles(ctx, h["1"], 0, 11, 15, 11);
  grid(ctx);
  marker(ctx, "figur", 7, 5);
  marker(ctx, "figur", 13, 8);
  marker(ctx, "schild", 7, 9);
};

/** Draws a picture; base64 of its PNG, or null where there is no canvas (a test without one). */
function draw(paint: Paint): string | null {
  if (typeof document === "undefined") return null;
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  let ctx: CanvasRenderingContext2D | null = null;
  try {
    ctx = canvas.getContext("2d");
  } catch {
    return null;
  }
  if (!ctx) return null;
  paint(ctx);
  const url = canvas.toDataURL("image/png");
  return url.startsWith(PREFIX) ? url.slice(PREFIX.length) : null;
}

export type ExamplePreviews = { own: string | null; standard: string | null };

/** The two example pictures: an own one (a route) and a standard one (a start town). */
export function drawExamplePreviews(): ExamplePreviews {
  return { own: draw(ROUTE), standard: draw(TOWN) };
}
