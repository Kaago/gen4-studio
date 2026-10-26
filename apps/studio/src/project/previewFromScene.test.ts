// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, PreviewDto } from "../api";
import { projectPreviewSet } from "../api";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { createTopDownRenderer } from "../world/thumbnailRenderer";
import { mapModelGlb } from "../world/thumbnailStore";
import { previewWritten } from "./previewMessages";
import { applyScenePreview, base64Of, mapOfCell, renderMapPicture, samePicture, setPreviewFrom3d, setPreviewFromMap } from "./previewFromScene";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), projectPreviewSet: vi.fn() }));
// No WebGL here: the map picture's renderer answers a fixed PNG and says which model it drew.
const drawn: ArrayBuffer[] = [];
vi.mock("../world/thumbnailRenderer", () => ({
  createTopDownRenderer: vi.fn(() => async (glb: ArrayBuffer) => {
    drawn.push(glb);
    return "data:image/png;base64,S0FSVEU=";
  }),
}));
vi.mock("../world/thumbnailStore", () => ({ mapModelGlb: vi.fn(async () => new ArrayBuffer(8)) }));

const written = (png: string): PreviewDto => ({ source: "eigen", png, standardMap: null });

beforeEach(() => {
  vi.mocked(projectPreviewSet).mockReset().mockImplementation(async ({ png }) => written(png ?? ""));
  drawn.length = 0;
  resetToasts();
});
afterEach(() => vi.unstubAllGlobals());

describe("preview from the scene", () => {
  it("base64_of_drops_the_data_url_prefix", () => {
    expect(base64Of("data:image/png;base64,UElD")).toBe("UElD");
    expect(base64Of("UElD")).toBe("UElD");
  });

  it("sets_the_3d_view_as_the_own_picture_and_says_so_to_the_open_preview", async () => {
    const heard: PreviewDto[] = [];
    const stop = previewWritten.subscribe((dto) => heard.push(dto));
    const host = { capture: vi.fn(() => "data:image/png;base64,M0Q=") };
    expect(await setPreviewFrom3d(host)).toEqual(written("M0Q="));
    stop();
    expect(host.capture).toHaveBeenCalledOnce();
    // The open project (no path), the own slot (default), the picture as it is: the kernel fits it to 256 × 192.
    expect(projectPreviewSet).toHaveBeenCalledWith({ png: "M0Q=" });
    expect(heard).toEqual([written("M0Q=")]);
  });

  it("draws_the_map_picture_in_256_by_192_with_its_own_renderer_and_sets_it", async () => {
    expect(await renderMapPicture(3, 7)).toBe("S0FSVEU=");
    expect(mapModelGlb).toHaveBeenCalledWith(3, 7);
    expect(await setPreviewFromMap(5, 2)).toEqual(written("S0FSVEU="));
    expect(mapModelGlb).toHaveBeenLastCalledWith(5, 2);
    expect(projectPreviewSet).toHaveBeenCalledWith({ png: "S0FSVEU=" });
    // One renderer of its own, made on the first picture, not the 64-pixel one of the map pictures.
    expect(createTopDownRenderer).toHaveBeenCalledTimes(1);
    expect(createTopDownRenderer).toHaveBeenCalledWith(256, 192);
    expect(drawn).toHaveLength(2);
  });

  it("finds_the_map_of_a_cell_and_the_tileset_of_its_header_as_the_world_map_does", () => {
    const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 2, maps: [10, null, 12, 13], headers: [411, 0, 412, 999] };
    const headers = [{ id: 411, area_data_id: 4 }, { id: 412, area_data_id: 5 }] as HeaderSummary[];
    expect(mapOfCell(matrix, 0, 0, headers, 1)).toEqual({ map: 10, areaDataId: 4 });
    expect(mapOfCell(matrix, 0, 1, headers, 1)).toEqual({ map: 12, areaDataId: 5 });
    // A header the list does not know, or a matrix without headers: the scene's tileset.
    expect(mapOfCell(matrix, 1, 1, headers, 1)).toEqual({ map: 13, areaDataId: 1 });
    expect(mapOfCell({ ...matrix, headers: null }, 0, 0, headers, 1)).toEqual({ map: 10, areaDataId: 1 });
    // An empty cell or one outside the matrix has no map.
    expect(mapOfCell(matrix, 1, 0, headers, 1)).toBeNull();
    expect(mapOfCell(matrix, 2, 0, headers, 1)).toBeNull();
    expect(mapOfCell(matrix, 0, 2, headers, 1)).toBeNull();
  });

  it("says_vorschaubild_gesetzt_or_the_sentence_of_the_refusal", async () => {
    const toasts = () => listToasts().map((t) => [t.kind, t.text]);
    expect(await applyScenePreview(async () => written("A"))).toEqual(written("A"));
    expect(toasts()).toEqual([["info", "Vorschaubild gesetzt"]]);
    resetToasts();
    expect(await applyScenePreview(async () => Promise.reject({ code: "error", message: "preview.unreadable: broken", asset: null }))).toBeNull();
    expect(toasts()).toEqual([["error", "Das Bild lässt sich nicht lesen. Wähle eine andere Datei."]]);
  });
});

describe("samePicture", () => {
  it("is_the_same_for_equal_text_without_decoding", async () => {
    expect(await samePicture("UElD", "UElD")).toBe(true);
  });

  it("is_not_the_same_when_the_pictures_cannot_be_decoded", async () => {
    // jsdom has no createImageBitmap: the pictures count as different, so the standard picture is written.
    expect(await samePicture("UElD", "UEle")).toBe(false);
  });

  it("compares_the_colours_of_the_pixels_and_not_the_encoding_or_alpha", async () => {
    // The kernel stores RGB8, the canvas gives RGBA: the same colours in another encoding are the same picture.
    const pixels: Record<string, number[]> = {
      // Two pixels each; a differs from b only in the alpha byte, c in a colour.
      a: [10, 20, 30, 255, 40, 50, 60, 255],
      b: [10, 20, 30, 254, 40, 50, 60, 0],
      c: [10, 20, 31, 255, 40, 50, 60, 255],
      wide: [10, 20, 30, 255, 40, 50, 60, 255, 0, 0, 0, 255],
    };
    vi.stubGlobal("createImageBitmap", async (blob: Blob) => {
      const key = await blob.text();
      return { width: pixels[key]!.length / 4, height: 1, key, close() {} };
    });
    let current = "";
    const getContext = vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ({
      drawImage: (bitmap: { key: string }) => {
        current = bitmap.key;
      },
      getImageData: () => ({ data: Uint8ClampedArray.from(pixels[current]!) }),
    })) as never);
    try {
      const b64 = (s: string) => btoa(s);
      expect(await samePicture(b64("a"), b64("b"))).toBe(true);
      expect(await samePicture(b64("a"), b64("c"))).toBe(false);
      expect(await samePicture(b64("a"), b64("wide"))).toBe(false);
    } finally {
      getContext.mockRestore();
    }
  });
});
