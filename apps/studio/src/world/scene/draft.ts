import * as THREE from "three";
import type { DraftDto, MatrixDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { MAP_SIZE, cellOrigin, cellY } from "../layout";
import { MARKER_Y, PLACEHOLDER_SIZE, SPRITE_TEXELS_PER_TILE, markerPlaceholder } from "../markers";
import { peekSprite, spriteTexture } from "../sprites";
import type { SpriteTexture } from "../sprites";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";

/** Opacity of everything the draft layer draws: planned, not there yet. */
export const DRAFT_OPACITY = 0.5;
/** Dash and gap of the planned cell and tile frames, in tiles. */
export const DRAFT_DASH = { dashSize: 1, gapSize: 0.5 };
/** The frames float this far over the overlay plane of their cell. */
const FRAME_LIFT = 0.1;

/** Height of matrix cell (`col`, `row`); a cell beside the matrix (a planned extension) has no altitude yet and lies on the ground. */
function cellHeight(matrix: MatrixDto, col: number, row: number): number {
  return col < matrix.width && row < matrix.height ? cellY(matrix, col, row) : 0;
}

/**
 * A dashed rectangle (four segments) from (`x0`, `z0`) to (`x1`, `z1`) at height `y`, in the selection colour at half opacity: the
 * selection colour dashed says „geplant“. Drawn over everything (no depth test) after the tiles.
 */
function dashedFrame(x0: number, z0: number, x1: number, z1: number, y: number): THREE.LineSegments {
  const geometry = new THREE.BufferGeometry();
  // Four segments, each a pair of points: north, east, south, west.
  geometry.setAttribute("position", new THREE.Float32BufferAttribute([
    x0, y, z0, x1, y, z0,
    x1, y, z0, x1, y, z1,
    x1, y, z1, x0, y, z1,
    x0, y, z1, x0, y, z0,
  ], 3));
  const material = new THREE.LineDashedMaterial({ color: szene.auswahl.halo, ...DRAFT_DASH, transparent: true, opacity: DRAFT_OPACITY, depthTest: false });
  const frame = new THREE.LineSegments(geometry, material);
  // The dashes are laid out along the distances of the line.
  frame.computeLineDistances();
  frame.renderOrder = 11;
  return frame;
}

/** The facing a planned figure is drawn in: the draft names no `dir`, so it looks down, as a figure stands by default. */
const DRAFT_FACING = 1;

/** Marks a marker whose picture belongs to the sprite cache (`world/sprites.ts`): clearing frees its material, never the picture. */
const SHARED_PICTURE = "draftSharedPicture";

/**
 * Gives `marker` the overworld sprite: the cache's texture at `width / 16 x height / 16` tiles, standing on the ground of its tile
 * (anchored at the bottom, like the markers of the scene). The placeholder it had is freed.
 */
function wearSprite(marker: THREE.Sprite, sprite: SpriteTexture, ground: number) {
  if (marker.userData[SHARED_PICTURE] !== true) marker.material.map?.dispose();
  marker.material.map = sprite.texture;
  marker.material.needsUpdate = true;
  marker.userData[SHARED_PICTURE] = true;
  marker.center.set(0.5, 0);
  marker.scale.set(sprite.width / SPRITE_TEXELS_PER_TILE, sprite.height / SPRITE_TEXELS_PER_TILE, 1);
  marker.position.y = ground;
}

/** Frees what `group` holds: each frame's geometry and material, each marker's material and its placeholder (a sprite's geometry is three's shared one, a figure's picture the sprite cache's). */
function clear(group: THREE.Group) {
  for (const child of [...group.children]) {
    group.remove(child);
    if (child instanceof THREE.Sprite) {
      if (child.userData[SHARED_PICTURE] !== true) child.material.map?.dispose();
      child.material.dispose();
    } else if (child instanceof THREE.LineSegments) {
      child.geometry.dispose();
      (child.material as THREE.Material).dispose();
    }
  }
}

/**
 * Replaces the draft layer of `h` with `draft` for `matrix` (null: nothing); returns how many cells, tiles and markers it drew. Does not
 * render. Cells lie on the matrix height, tiles and markers on the ground of the scene (`h.ground`, which belongs to the shown `matrix`).
 */
export function setDraft(h: SceneHost, draft: DraftDto | null, matrix: MatrixDto | null): number {
  clear(h.draft);
  let drawn = 0;
  if (draft && matrix) {
    const on = (item: { matrix: number | null }) => item.matrix === matrix.index;
    for (const cell of draft.cells.filter(on)) {
      const { x, z } = cellOrigin(cell.x, cell.y);
      h.draft.add(dashedFrame(x, z, x + MAP_SIZE, z + MAP_SIZE, OVERLAY_Y + cellHeight(matrix, cell.x, cell.y) + FRAME_LIFT));
      drawn++;
    }
    for (const tile of draft.tiles.filter(on)) {
      h.draft.add(dashedFrame(tile.x, tile.z, tile.x + 1, tile.z + 1, OVERLAY_Y + h.ground.y(tile.x, tile.z) + FRAME_LIFT));
      drawn++;
    }
    for (const event of draft.events.filter(on)) {
      // The placeholder of the marker's kind; a figure with a `look` wears its overworld sprite once the cache has it.
      const ground = h.ground.y(event.x, event.z);
      const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: markerPlaceholder(event.kind), transparent: true, opacity: DRAFT_OPACITY, depthTest: false }));
      sprite.scale.set(PLACEHOLDER_SIZE.width, PLACEHOLDER_SIZE.height, 1);
      sprite.position.set(event.x + 0.5, MARKER_Y + ground, event.z + 0.5);
      sprite.renderOrder = 11;
      h.draft.add(sprite);
      drawn++;
      if (event.look !== null) dressInSprite(h, sprite, event.look, ground);
    }
  }
  h.draftCount = drawn;
  return drawn;
}

/**
 * The overworld sprite of `look` for a planned figure: at once when the cache holds it, else when it has loaded (then the scene is
 * drawn again). A marker the layer dropped meanwhile (the draft changed) stays as it is; a look without a sprite keeps the placeholder.
 */
function dressInSprite(h: SceneHost, marker: THREE.Sprite, look: number, ground: number) {
  const known = peekSprite(look, DRAFT_FACING);
  if (known) {
    wearSprite(marker, known, ground);
    return;
  }
  if (known === null) return;
  void spriteTexture(look, DRAFT_FACING).then((sprite) => {
    if (!sprite || marker.parent !== h.draft) return;
    wearSprite(marker, sprite, ground);
    h.render();
  });
}
