import * as THREE from "three";
import { getObjectSprite } from "../api";

/** The facing a `dir` value shows: 0 up, 1 down, 2 left, 3 right; anything else faces down. */
export type Facing = 0 | 1 | 2 | 3;

export function facingOf(dir: number): Facing {
  return dir === 0 || dir === 1 || dir === 2 || dir === 3 ? dir : 1;
}

/** A loaded overworld sprite: the texture and its size in texels. */
export interface SpriteTexture {
  texture: THREE.Texture;
  width: number;
  height: number;
}

/**
 * The sprite textures of the open project, by graphics id and facing. The cache owns every texture it hands out
 * (marker sets only dispose their own materials) and disposes them in `resetSprites`.
 * Concurrent asks share one request; a sprite the game does not have (`null`) is remembered like any other
 * result, a failed request is not.
 */
const cache = new Map<string, Promise<SpriteTexture | null>>();
/** The settled entries of `cache`, for a synchronous look. */
const settled = new Map<string, SpriteTexture | null>();
/** Bumped by `resetSprites`; a request started before it belongs to the previous project. */
let generation = 0;

const keyOf = (graphicsId: number, facing: Facing) => `${graphicsId}:${facing}`;

/** An upright texture of RGBA rows (row 0 on top): the GPU takes the bottom row first, so the rows are reversed. */
function toTexture(width: number, height: number, rgba: ArrayLike<number>): THREE.DataTexture | null {
  const row = width * 4;
  if (width < 1 || height < 1 || rgba.length !== row * height) return null;
  const data = new Uint8Array(rgba.length);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < row; x++) data[(height - 1 - y) * row + x] = rgba[y * row + x];
  }
  const texture = new THREE.DataTexture(data, width, height, THREE.RGBAFormat);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.generateMipmaps = false;
  texture.colorSpace = THREE.SRGBColorSpace;
  // The rows are already stored bottom first, so no flip on upload.
  texture.flipY = false;
  texture.needsUpdate = true;
  return texture;
}

/** The texture of the overworld sprite of `graphicsId` facing `dir`, or null when the game has none (or it could not be read). */
export function spriteTexture(graphicsId: number, dir: number): Promise<SpriteTexture | null> {
  const facing = facingOf(dir);
  const key = keyOf(graphicsId, facing);
  const known = cache.get(key);
  if (known) return known;
  const started = generation;
  const request = getObjectSprite(graphicsId, facing).then(
    (dto) => {
      const texture = dto ? toTexture(dto.width, dto.height, dto.rgba) : null;
      if (started !== generation) {
        // The project changed while the request ran: nobody owns this texture any more.
        texture?.dispose();
        return null;
      }
      const result = texture && dto ? { texture, width: dto.width, height: dto.height } : null;
      settled.set(key, result);
      return result;
    },
    () => {
      // A failure is not remembered: the next ask tries again.
      if (cache.get(key) === request) cache.delete(key);
      return null;
    },
  );
  cache.set(key, request);
  return request;
}

/**
 * The sprite if it has already loaded (`null`: the game has none), `undefined` while it is unknown or loading;
 * lets a rebuilt marker show its sprite at once instead of flashing the placeholder.
 */
export function peekSprite(graphicsId: number, dir: number): SpriteTexture | null | undefined {
  return settled.get(keyOf(graphicsId, facingOf(dir)));
}

/** Forgets every sprite and disposes the textures (another project was opened). */
export function resetSprites() {
  generation++;
  for (const result of settled.values()) result?.texture.dispose();
  settled.clear();
  cache.clear();
}
