import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({ getTilesetTexture: vi.fn() }));
vi.mock("../../api.world", () => api);

import { peekTileImage, resetTileImages, tileImage } from "./paletteImages";

/** A stand-in bitmap: what `createImageBitmap` was handed, and whether it was closed. */
class FakeBitmap {
  closed = false;
  constructor(readonly data: { width: number; height: number }) {}
  close() {
    this.closed = true;
  }
}

describe("paletteImages", () => {
  beforeEach(() => {
    resetTileImages();
    api.getTilesetTexture.mockReset().mockImplementation(async () => ({ width: 2, height: 1, rgba: [255, 0, 0, 255, 0, 255, 0, 255] }));
    vi.stubGlobal(
      "ImageData",
      class {
        constructor(
          readonly data: Uint8ClampedArray,
          readonly width: number,
          readonly height: number,
        ) {}
      },
    );
    vi.stubGlobal("createImageBitmap", vi.fn(async (data: { width: number; height: number }) => new FakeBitmap(data)));
  });
  afterEach(() => vi.unstubAllGlobals());

  it("images_are_cached_per_package_and_texture", async () => {
    const a = await tileImage(3, "gras_a");
    const again = await tileImage(3, "gras_a");
    expect(a).toBe(again);
    expect(api.getTilesetTexture).toHaveBeenCalledTimes(1);
    expect(api.getTilesetTexture).toHaveBeenCalledWith(3, "gras_a");
    expect((a as unknown as FakeBitmap).data.width).toBe(2);
    expect(peekTileImage(3, "gras_a")).toBe(a);

    // Another package or texture is its own picture.
    await tileImage(4, "gras_a");
    await tileImage(3, "weg");
    expect(api.getTilesetTexture).toHaveBeenCalledTimes(3);

    // Another project: the pictures go (closed) and are asked for anew.
    resetTileImages();
    expect((a as unknown as FakeBitmap).closed).toBe(true);
    expect(peekTileImage(3, "gras_a")).toBeUndefined();
    await tileImage(3, "gras_a");
    expect(api.getTilesetTexture).toHaveBeenCalledTimes(4);
  });

  it("null_stays_null", async () => {
    api.getTilesetTexture.mockImplementation(async () => null);
    expect(await tileImage(3, "wasser_a")).toBeNull();
    expect(await tileImage(3, "wasser_a")).toBeNull();
    expect(api.getTilesetTexture).toHaveBeenCalledTimes(1);
    expect(peekTileImage(3, "wasser_a")).toBeNull();

    // A failed request is not remembered: the next ask tries again.
    api.getTilesetTexture.mockImplementationOnce(async () => Promise.reject(new Error("weg")));
    expect(await tileImage(3, "fels")).toBeNull();
    expect(await tileImage(3, "fels")).toBeNull();
    expect(api.getTilesetTexture).toHaveBeenCalledTimes(3);
  });
});
