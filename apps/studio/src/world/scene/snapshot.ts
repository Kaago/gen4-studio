/**
 * A small picture of a canvas („Als Vorschaubild“ from the 3D view, phase 4.0 §3.5): the canvas cover-cropped to the picture's shape and
 * drawn into a 2D canvas of that size, which alone is encoded. The full canvas of a 5K or 6K display has up to 20 megapixels; encoding it
 * as PNG would block the window and could pass the kernel's 32 MiB.
 */

import { t } from "../../i18n";

/** The size of the picture taken from the 3D view: 4 : 3 like the preview, twice its 256 × 192 (the kernel fits it). */
export const SNAPSHOT_W = 512;
export const SNAPSHOT_H = 384;

/** The part of a `w` × `h` picture that fills a `frameW` × `frameH` frame without bars: its centre, cut on the side that is too long. */
export function coverRect(w: number, h: number, frameW: number, frameH: number): { sx: number; sy: number; sw: number; sh: number } {
  if (w * frameH > h * frameW) {
    const sw = (h * frameW) / frameH;
    return { sx: (w - sw) / 2, sy: 0, sw, sh: h };
  }
  const sh = (w * frameH) / frameW;
  return { sx: 0, sy: (h - sh) / 2, sw: w, sh };
}

/**
 * Draws `source` (its drawing buffer: `width` × `height` pixels) cover-cropped into a new canvas of `width` × `height` and answers that as a
 * PNG data URL. For a WebGL canvas without a kept drawing buffer, call it in the same tick as the render.
 */
export function snapshot(source: HTMLCanvasElement, width = SNAPSHOT_W, height = SNAPSHOT_H): string {
  const out = document.createElement("canvas");
  out.width = width;
  out.height = height;
  const ctx = out.getContext("2d");
  if (!ctx) throw new Error(t("world:snapshot.noContext"));
  const { sx, sy, sw, sh } = coverRect(source.width, source.height, width, height);
  ctx.drawImage(source, sx, sy, sw, sh, 0, 0, width, height);
  return out.toDataURL("image/png");
}
