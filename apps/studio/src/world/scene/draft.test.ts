import * as THREE from "three";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DraftDto, MatrixDto } from "../../api";
import { draftCount } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { MARKER_Y, PLACEHOLDER_SIZE, SPRITE_TEXELS_PER_TILE } from "../markers";
import { peekSprite, spriteTexture } from "../sprites";
import type { SpriteTexture } from "../sprites";
import { OVERLAY_Y } from "./content";
import { DRAFT_DASH, DRAFT_OPACITY, setDraft } from "./draft";
import type { SceneHost } from "./SceneHost";

// The sprite cache asks the backend; here each test says what it holds.
vi.mock("../sprites", () => ({ peekSprite: vi.fn(), spriteTexture: vi.fn() }));

/** Matrix 0, 3 × 2, cell (1, 1) raised by 4 altitude steps (2 scene units). */
const matrix0: MatrixDto = { index: 0, name: "m", width: 3, height: 2, maps: [1, 2, 3, 4, 5, 6], headers: null, altitudes: [0, 0, 0, 0, 4, 0] };

const draft: DraftDto = {
  cells: [
    { matrix: 0, x: 1, y: 1, source_map: 9 },
    { matrix: null, x: 0, y: 0, source_map: 9 },
  ],
  tiles: [{ matrix: 0, x: 40, z: 33 }],
  events: [
    { kind: "warp", matrix: 0, x: 40, z: 33, look: null, target: { header: 7, warp: 0 } },
    { kind: "object", matrix: 3, x: 5, z: 5, look: 12, target: null },
  ],
  messages: [],
  notes: [],
  irreversible: [],
};

/** The part of the host the draft layer lives on. */
const fakeHost = () => ({ draft: new THREE.Group(), draftCount: 0 }) as unknown as SceneHost;

/** The y of every vertex of a line frame. */
const heights = (line: THREE.LineSegments) => {
  const position = line.geometry.getAttribute("position");
  return [...new Set(Array.from({ length: position.count }, (_, i) => position.getY(i)))];
};

describe("setDraft", () => {
  beforeEach(() => {
    vi.mocked(peekSprite).mockReset().mockReturnValue(undefined);
    vi.mocked(spriteTexture).mockReset().mockResolvedValue(null);
  });

  it("draws_cells_tiles_and_events_of_the_shown_matrix_only", () => {
    const h = fakeHost();
    expect(setDraft(h, draft, matrix0)).toBe(3);
    expect(h.draft.children).toHaveLength(3);
    expect(h.draftCount).toBe(3);
    expect(draftCount(draft, 0)).toBe(3);
    const [cell, tile, marker] = h.draft.children as [THREE.LineSegments, THREE.LineSegments, THREE.Sprite];
    // The cell (1, 1) spans x 32..64, z 32..64: four segments around it, just over its overlay plane at its altitude.
    expect(cell).toBeInstanceOf(THREE.LineSegments);
    const position = cell.geometry.getAttribute("position");
    expect(position.count).toBe(8);
    const xs = Array.from({ length: 8 }, (_, i) => position.getX(i));
    const zs = Array.from({ length: 8 }, (_, i) => position.getZ(i));
    expect([Math.min(...xs), Math.max(...xs), Math.min(...zs), Math.max(...zs)]).toEqual([32, 64, 32, 64]);
    expect(heights(cell)).toEqual([Math.fround(OVERLAY_Y + 2 + 0.1)]);
    expect(cell.renderOrder).toBe(11);
    // The door tile (40, 33) lies in cell (1, 1): a 1 × 1 frame at the same height.
    expect(tile).toBeInstanceOf(THREE.LineSegments);
    const t = tile.geometry.getAttribute("position");
    const txs = Array.from({ length: t.count }, (_, i) => t.getX(i));
    const tzs = Array.from({ length: t.count }, (_, i) => t.getZ(i));
    expect([Math.min(...txs), Math.max(...txs), Math.min(...tzs), Math.max(...tzs)]).toEqual([40, 41, 33, 34]);
    expect(heights(tile)).toEqual([Math.fround(OVERLAY_Y + 2 + 0.1)]);
    // The warp: a half-transparent placeholder sprite in the warp's colour over the tile's centre.
    expect(marker).toBeInstanceOf(THREE.Sprite);
    expect(marker.position.toArray()).toEqual([40.5, MARKER_Y + 2, 33.5]);
    expect(marker.material.opacity).toBe(DRAFT_OPACITY);
    expect(marker.material.transparent).toBe(true);
    expect(marker.material.map).toBeInstanceOf(THREE.DataTexture);
    expect(marker.renderOrder).toBe(11);

    // Null clears the layer and frees what it drew.
    const disposed = vi.fn();
    const sharedGeometry = vi.fn();
    for (const child of h.draft.children as (THREE.LineSegments | THREE.Sprite)[]) {
      child.geometry.addEventListener("dispose", child instanceof THREE.Sprite ? sharedGeometry : disposed);
      (child.material as THREE.Material).addEventListener("dispose", disposed);
    }
    const texture = marker.material.map!;
    texture.addEventListener("dispose", disposed);
    expect(setDraft(h, null, matrix0)).toBe(0);
    expect(h.draft.children).toHaveLength(0);
    expect(h.draftCount).toBe(0);
    // The two frames' geometries, the three materials, the placeholder texture; a sprite's geometry is three's shared one and stays.
    expect(disposed).toHaveBeenCalledTimes(6);
    expect(sharedGeometry).not.toHaveBeenCalled();
  });

  it("draws_nothing_without_a_matrix_and_replaces_the_layer_before", () => {
    const h = fakeHost();
    setDraft(h, draft, matrix0);
    expect(setDraft(h, draft, null)).toBe(0);
    expect(h.draft.children).toHaveLength(0);
    // Matrix 3 has only the object.
    expect(setDraft(h, draft, { ...matrix0, index: 3 })).toBe(1);
    expect(h.draft.children).toHaveLength(1);
    expect(setDraft(h, draft, matrix0)).toBe(3);
    expect(h.draft.children).toHaveLength(3);
  });

  it("a_dashed_material_with_half_opacity", () => {
    const h = fakeHost();
    setDraft(h, draft, matrix0);
    for (const frame of h.draft.children.slice(0, 2) as THREE.LineSegments[]) {
      const material = frame.material as THREE.LineDashedMaterial;
      expect(material).toBeInstanceOf(THREE.LineDashedMaterial);
      expect(material.opacity).toBe(0.5);
      expect(material.transparent).toBe(true);
      expect(material.depthTest).toBe(false);
      expect(material.dashSize).toBe(DRAFT_DASH.dashSize);
      expect(material.gapSize).toBe(DRAFT_DASH.gapSize);
      expect(material.color.getHex()).toBe(new THREE.Color(szene.auswahl.halo).getHex());
      // `computeLineDistances` was called: the dashes need the distances along the line.
      expect(frame.geometry.attributes.lineDistance).toBeDefined();
    }
    expect(DRAFT_DASH).toEqual({ dashSize: 1, gapSize: 0.5 });
  });

  it("draws_a_cell_outside_the_matrix_on_the_ground", () => {
    // „Gebiet erweitern“ past the edge: the cell (3, 0) lies beside the matrix; it has no altitude yet.
    const h = fakeHost();
    const beside: DraftDto = { ...draft, cells: [{ matrix: 0, x: 3, y: 0, source_map: 9 }], tiles: [], events: [] };
    expect(setDraft(h, beside, matrix0)).toBe(1);
    expect(heights(h.draft.children[0] as THREE.LineSegments)).toEqual([Math.fround(OVERLAY_Y + 0.1)]);
  });

  it("an_event_with_a_look_draws_its_sprite", async () => {
    const tick = () => new Promise<void>((resolve) => setTimeout(resolve, 0));
    const figure: DraftDto = { ...draft, cells: [], tiles: [], events: [{ kind: "object", matrix: 0, x: 40, z: 33, look: 6, target: null }] };
    const loaded = (width: number, height: number): SpriteTexture => ({ texture: new THREE.DataTexture(new Uint8Array(width * height * 4), width, height), width, height });
    const host = () => ({ draft: new THREE.Group(), draftCount: 0, render: vi.fn() }) as unknown as SceneHost & { render: ReturnType<typeof vi.fn> };

    // A sprite the cache already holds: at once, at its size in tiles, standing on the tile's ground, half transparent.
    const known = loaded(32, 32);
    vi.mocked(peekSprite).mockReturnValue(known);
    const h = host();
    expect(setDraft(h, figure, matrix0)).toBe(1);
    const sprite = h.draft.children[0] as THREE.Sprite;
    expect(peekSprite).toHaveBeenCalledWith(6, 1);
    expect(sprite.material.map).toBe(known.texture);
    expect(sprite.material.opacity).toBe(DRAFT_OPACITY);
    expect(sprite.scale.toArray()).toEqual([32 / SPRITE_TEXELS_PER_TILE, 32 / SPRITE_TEXELS_PER_TILE, 1]);
    expect(sprite.center.toArray()).toEqual([0.5, 0]);
    expect(sprite.position.toArray()).toEqual([40.5, 2, 33.5]);
    expect(spriteTexture).not.toHaveBeenCalled();
    // The texture belongs to the sprite cache: clearing the layer frees the material, not the picture.
    const freed = vi.fn();
    known.texture.addEventListener("dispose", freed);
    setDraft(h, null, matrix0);
    expect(freed).not.toHaveBeenCalled();

    // Not loaded yet: the placeholder until the sprite comes, then the sprite, and the scene is drawn again.
    vi.mocked(peekSprite).mockReturnValue(undefined);
    let arrive: (sprite: SpriteTexture | null) => void = () => {};
    vi.mocked(spriteTexture).mockReturnValue(new Promise((resolve) => (arrive = resolve)));
    const later = host();
    setDraft(later, figure, matrix0);
    const waiting = later.draft.children[0] as THREE.Sprite;
    const placeholder = waiting.material.map!;
    expect(spriteTexture).toHaveBeenCalledWith(6, 1);
    expect(placeholder).not.toBe(known.texture);
    expect(waiting.scale.toArray()).toEqual([PLACEHOLDER_SIZE.width, PLACEHOLDER_SIZE.height, 1]);
    expect(waiting.position.toArray()).toEqual([40.5, MARKER_Y + 2, 33.5]);
    const placeholderFreed = vi.fn();
    placeholder.addEventListener("dispose", placeholderFreed);
    const tall = loaded(32, 48);
    arrive(tall);
    await tick();
    expect(waiting.material.map).toBe(tall.texture);
    expect(waiting.scale.toArray()).toEqual([2, 3, 1]);
    expect(waiting.position.toArray()).toEqual([40.5, 2, 33.5]);
    expect(placeholderFreed).toHaveBeenCalledOnce();
    expect(later.render).toHaveBeenCalledOnce();

    // A look the game has no sprite for keeps the placeholder, without an error.
    vi.mocked(spriteTexture).mockResolvedValue(null);
    const none = host();
    setDraft(none, figure, matrix0);
    const kept = (none.draft.children[0] as THREE.Sprite).material.map;
    await tick();
    expect((none.draft.children[0] as THREE.Sprite).material.map).toBe(kept);
    expect(none.render).not.toHaveBeenCalled();

    // A sprite that comes after the draft changed touches nothing (its marker is gone).
    let late: (sprite: SpriteTexture | null) => void = () => {};
    vi.mocked(spriteTexture).mockReturnValue(new Promise((resolve) => (late = resolve)));
    const moved = host();
    setDraft(moved, figure, matrix0);
    const gone = moved.draft.children[0] as THREE.Sprite;
    setDraft(moved, null, matrix0);
    late(loaded(16, 16));
    await tick();
    expect(gone.material.map).not.toBe(null);
    expect(gone.scale.toArray()).toEqual([PLACEHOLDER_SIZE.width, PLACEHOLDER_SIZE.height, 1]);
    expect(moved.render).not.toHaveBeenCalled();
  });
});
