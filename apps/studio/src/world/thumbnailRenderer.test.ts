// @vitest-environment jsdom
import * as THREE from "three";
import { beforeEach, describe, expect, it, vi } from "vitest";

/** What the stand-in WebGL renderer was asked: its size and the camera of each render. */
const sizes: [number, number][] = [];
const cameras: THREE.OrthographicCamera[] = [];

vi.mock("three", async (importOriginal) => {
  const three = await importOriginal<typeof import("three")>();
  class FakeRenderer {
    domElement = { toDataURL: () => "data:image/png;base64,UElD" };
    setPixelRatio() {}
    setClearColor() {}
    setSize(w: number, h: number) {
      sizes.push([w, h]);
    }
    render(_scene: unknown, camera: THREE.OrthographicCamera) {
      cameras.push(camera.clone());
    }
    getContext() {
      return { isContextLost: () => false };
    }
    dispose() {}
  }
  return { ...three, WebGLRenderer: FakeRenderer };
});
// A map model of one box, 512 units wide (a whole map) and 40 high; no GLB is parsed.
vi.mock("./sceneUtil", async () => {
  const three = await import("three");
  return {
    parseGlb: async () => {
      const group = new three.Group();
      group.add(new three.Mesh(new three.BoxGeometry(512, 40, 512)));
      return group;
    },
    drawDoubleSided: () => {},
    disposeTree: () => {},
  };
});

import { MAP_SIZE, MODEL_UNITS_PER_TILE } from "./layout";
import { createTopDownRenderer } from "./thumbnailRenderer";

const MAP_WIDTH = MAP_SIZE * MODEL_UNITS_PER_TILE;

beforeEach(() => {
  sizes.length = 0;
  cameras.length = 0;
});

describe("createTopDownRenderer", () => {
  it("draws_a_square_picture_of_the_whole_map_by_default", async () => {
    const render = createTopDownRenderer();
    expect(await render(new ArrayBuffer(4))).toBe("data:image/png;base64,UElD");
    expect(sizes).toEqual([[64, 64]]);
    const camera = cameras[0]!;
    expect(camera.right - camera.left).toBe(MAP_WIDTH);
    expect(camera.top - camera.bottom).toBe(MAP_WIDTH);
  });

  it("draws_256_by_192_with_the_map_width_filling_the_picture_and_top_and_bottom_cut", async () => {
    const render = createTopDownRenderer(256, 192);
    await render(new ArrayBuffer(4));
    await render(new ArrayBuffer(4));
    // One renderer for every call, sized once.
    expect(sizes).toEqual([[256, 192]]);
    for (const camera of cameras) {
      expect(camera.right - camera.left).toBe(MAP_WIDTH);
      expect(camera.top - camera.bottom).toBe(0.75 * (camera.right - camera.left));
      // Centred on the map: an eighth of the map's depth is cut at the top and at the bottom.
      expect(camera.top).toBe(-camera.bottom);
    }
  });
});
