import * as THREE from "three";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getObjectSprite } from "../api";
import type { SpriteDto } from "../api";
import { facingOf, peekSprite, resetSprites, spriteTexture } from "./sprites";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getObjectSprite: vi.fn() }));

/** A 2 x 3 sprite whose rows are told apart by their first byte (row 0 is the top). */
const sprite = (): SpriteDto => ({
  width: 2,
  height: 3,
  rgba: [0, 0, 0, 255, 0, 0, 0, 255, 1, 0, 0, 255, 1, 0, 0, 255, 2, 0, 0, 255, 2, 0, 0, 255],
});

beforeEach(() => {
  resetSprites();
  vi.mocked(getObjectSprite).mockReset().mockResolvedValue(null);
});

describe("facingOf", () => {
  it("names the four directions and shows anything else facing down", () => {
    expect([0, 1, 2, 3].map(facingOf)).toEqual([0, 1, 2, 3]);
    expect([-1, 4, 255, 1.5].map(facingOf)).toEqual([1, 1, 1, 1]);
  });
});

describe("spriteTexture", () => {
  it("unknown_graphics_id_keeps_the_yellow_marker", async () => {
    expect(await spriteTexture(900, 1)).toBeNull();
    expect(await spriteTexture(900, 1)).toBeNull();
    // The miss is remembered: the backend is asked once per (graphics id, facing).
    expect(getObjectSprite).toHaveBeenCalledTimes(1);
    expect(getObjectSprite).toHaveBeenCalledWith(900, 1);
    expect(peekSprite(900, 1)).toBeNull();
  });

  it("asks again after a failure, and does not remember it", async () => {
    vi.mocked(getObjectSprite).mockRejectedValueOnce(new Error("boom"));
    expect(await spriteTexture(3, 0)).toBeNull();
    vi.mocked(getObjectSprite).mockResolvedValueOnce(sprite());
    expect(await spriteTexture(3, 0)).not.toBeNull();
    expect(getObjectSprite).toHaveBeenCalledTimes(2);
  });

  it("makes one request per graphics id and facing, however often and however concurrently it is asked", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue(sprite());
    const [a, b] = await Promise.all([spriteTexture(5, 2), spriteTexture(5, 2)]);
    expect(a).toBe(b);
    expect(await spriteTexture(5, 2)).toBe(a);
    // Directions that show down share a request.
    await spriteTexture(5, 7);
    await spriteTexture(5, 1);
    expect(getObjectSprite).toHaveBeenCalledTimes(2);
    expect(vi.mocked(getObjectSprite).mock.calls).toEqual([
      [5, 2],
      [5, 1],
    ]);
  });

  it("makes a nearest-filtered sRGB texture that shows the first row of the sprite on top", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue(sprite());
    const tex = await spriteTexture(1, 1);
    expect(tex).toMatchObject({ width: 2, height: 3 });
    const t = tex?.texture as THREE.DataTexture;
    expect(t.magFilter).toBe(THREE.NearestFilter);
    expect(t.minFilter).toBe(THREE.NearestFilter);
    expect(t.colorSpace).toBe(THREE.SRGBColorSpace);
    expect(t.generateMipmaps).toBe(false);
    // Texture rows are stored bottom first (v = 0 is the bottom of a sprite quad), so the sprite's
    // top row comes last; flipY stays off, the rows are already in the order the GPU wants.
    expect(t.flipY).toBe(false);
    const data = t.image.data as Uint8Array;
    expect([data[0], data[8], data[16]]).toEqual([2, 1, 0]);
    expect(Array.from(data.slice(0, 4))).toEqual([2, 0, 0, 255]);
  });

  it("treats data of the wrong size as no sprite", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue({ width: 2, height: 2, rgba: [1, 2, 3] });
    expect(await spriteTexture(8, 1)).toBeNull();
  });
});

describe("peekSprite", () => {
  it("hands out a texture only once it has loaded", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue(sprite());
    expect(peekSprite(1, 1)).toBeUndefined();
    const pending = spriteTexture(1, 1);
    expect(peekSprite(1, 1)).toBeUndefined();
    expect(peekSprite(1, 1)).toBeUndefined();
    const tex = await pending;
    expect(peekSprite(1, 1)).toBe(tex);
  });
});

describe("resetSprites", () => {
  it("forgets everything and disposes the textures it owns", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue(sprite());
    const tex = await spriteTexture(1, 1);
    const disposed = vi.spyOn(tex?.texture as THREE.Texture, "dispose");
    resetSprites();
    expect(disposed).toHaveBeenCalled();
    expect(peekSprite(1, 1)).toBeUndefined();
    await spriteTexture(1, 1);
    expect(getObjectSprite).toHaveBeenCalledTimes(2);
  });

  it("drops a sprite that arrives after the reset (it belongs to the previous project)", async () => {
    let answer: (s: SpriteDto) => void = () => undefined;
    vi.mocked(getObjectSprite).mockReturnValue(new Promise((resolve) => (answer = resolve)));
    const pending = spriteTexture(1, 1);
    resetSprites();
    answer(sprite());
    expect(await pending).toBeNull();
    expect(peekSprite(1, 1)).toBeUndefined();
  });
});
