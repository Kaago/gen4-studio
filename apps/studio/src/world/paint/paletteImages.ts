import { getTilesetTexture } from "../../api.world";
import type { SpriteDto } from "../../api";

/**
 * The pictures of the palette's tiles (the whole texture of a tile, `get_tileset_texture`), by package and texture name, as bitmaps:
 * the palette draws them, the stroke preview lays them on its plates. Concurrent asks share one request; a texture the package has no
 * picture of (`null`: a synthetic tileset, a format without decoder) is remembered, a failed request is not. `resetTileImages` forgets
 * them all (another project, or the package's files were written).
 */
const images = new Map<string, Promise<ImageBitmap | null>>();
/** The answers already there, for the preview, which cannot wait. */
const settled = new Map<string, ImageBitmap | null>();
/** Bumped by `resetTileImages`; a request started before it belongs to the previous project. */
let generation = 0;

const keyOf = (pkg: number, texture: string) => `${pkg}:${texture}`;

/** The RGBA pixels of `dto` as a bitmap (pixel for pixel: the palette and the preview scale it with the nearest neighbour). */
async function bitmapOf(dto: SpriteDto): Promise<ImageBitmap | null> {
  if (typeof createImageBitmap !== "function" || typeof ImageData === "undefined") return null;
  return createImageBitmap(new ImageData(new Uint8ClampedArray(dto.rgba), dto.width, dto.height));
}

/** The texture of a tile as a bitmap (cached per package and texture; null without picture). */
export function tileImage(pkg: number, texture: string): Promise<ImageBitmap | null> {
  const key = keyOf(pkg, texture);
  const known = images.get(key);
  if (known) return known;
  const started = generation;
  const request = getTilesetTexture(pkg, texture)
    .then((dto) => (dto ? bitmapOf(dto) : null))
    .then(
      (made) => {
        if (started !== generation) {
          made?.close();
          return null;
        }
        settled.set(key, made);
        return made;
      },
      () => {
        if (images.get(key) === request) images.delete(key);
        return null;
      },
    );
  images.set(key, request);
  return request;
}

/** The picture of a tile if its answer is already there: the bitmap, null without picture, undefined while unknown. */
export function peekTileImage(pkg: number, texture: string): ImageBitmap | null | undefined {
  return settled.get(keyOf(pkg, texture));
}

/** Forgets every picture and frees the bitmaps (on project switch, like `resetWorldTextures`). */
export function resetTileImages(): void {
  generation++;
  for (const bitmap of settled.values()) bitmap?.close();
  settled.clear();
  images.clear();
}
