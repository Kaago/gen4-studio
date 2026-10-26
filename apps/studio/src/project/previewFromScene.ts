import { projectPreviewSet } from "../api";
import type { HeaderSummary, MatrixDto, PreviewDto } from "../api";
import { toast } from "../ui/frame/toasts";
import type { SceneHost } from "../world/scene/SceneHost";
import { createTopDownRenderer } from "../world/thumbnailRenderer";
import type { ThumbnailRender } from "../world/thumbnails";
import { mapModelGlb } from "../world/thumbnailStore";
import { previewErrorText, previewWritten } from "./previewMessages";

/**
 * The project's preview picture from the scene (phase 4.0 §3.4, §3.5): the 3D view as it shows, or a map's picture drawn from straight
 * above at the preview's size (the start town's for the standard picture). The kernel fits whatever it gets to 256 × 192.
 */

/** The preview's size; the map picture is drawn at it, so the kernel stores it as drawn. */
const PREVIEW_W = 256;
const PREVIEW_H = 192;

/** The base64 of a data URL (what `project_preview_set` takes as `png`); a string without the prefix stays as it is. */
export function base64Of(dataUrl: string): string {
  const comma = dataUrl.indexOf(",");
  return dataUrl.startsWith("data:") && comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
}

/**
 * The renderer of the preview's map pictures: its own, made on the first picture, apart from the 64-pixel one of the map pictures' cache.
 * It lives for the whole session (never disposed, also across projects). After its WebGL context was lost `MAX_CONTEXTS` times
 * (`createTopDownRenderer`) every further picture throws: the standard picture then fails silently until the app restarts, and „Als
 * Vorschaubild“ from the world map says the error.
 */
let mapRenderer: ThumbnailRender | null = null;

/** Draws the picture of `map` with the tileset `areaDataId` at 256 × 192 (the map's width fills it) and answers its base64. */
export async function renderMapPicture(map: number, areaDataId: number): Promise<string> {
  mapRenderer ??= createTopDownRenderer(PREVIEW_W, PREVIEW_H);
  const glb = await mapModelGlb(map, areaDataId);
  return base64Of(await mapRenderer(glb));
}

/** Writes `png` as the open project's own picture and tells the open preview; answers the preview after the write. */
async function setOwn(png: string): Promise<PreviewDto> {
  const preview = await projectPreviewSet({ png });
  previewWritten.emit(preview);
  return preview;
}

/** „Als Vorschaubild“ on the 3D view: its picture now (`SceneHost.capture`: 4 : 3 at 512 × 384, not the full canvas) becomes the own picture. */
export async function setPreviewFrom3d(host: Pick<SceneHost, "capture">): Promise<PreviewDto> {
  return setOwn(base64Of(host.capture()));
}

/** „Als Vorschaubild“ on the world map: the picture of the chosen map, drawn at 256 × 192, becomes the own picture. */
export async function setPreviewFromMap(map: number, areaDataId: number): Promise<PreviewDto> {
  return setOwn(await renderMapPicture(map, areaDataId));
}

/**
 * The map at cell (`x`, `y`) of `matrix` and its tileset as the world map draws it: the tileset of the cell's header, else `fallbackArea`
 * (the scene's). Null for an empty cell or one outside the matrix.
 */
export function mapOfCell(matrix: MatrixDto, x: number, y: number, headers: readonly HeaderSummary[], fallbackArea: number): { map: number; areaDataId: number } | null {
  if (x < 0 || y < 0 || x >= matrix.width || y >= matrix.height) return null;
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  if (map === null) return null;
  const header = matrix.headers?.[i];
  const areaDataId = header === undefined ? undefined : headers.find((h) => h.id === header)?.area_data_id;
  return { map, areaDataId: areaDataId ?? fallbackArea };
}

/** Runs a write of the preview from the scene and says so: „Vorschaubild gesetzt“, or the sentence of the refusal. Null when it failed (never rejects). */
export async function applyScenePreview(write: () => Promise<PreviewDto>): Promise<PreviewDto | null> {
  try {
    const preview = await write();
    toast.info("Vorschaubild gesetzt");
    return preview;
  } catch (e) {
    toast.error(previewErrorText(e));
    return null;
  }
}

/** The pixels of a PNG (base64) as RGBA, or null where pictures cannot be decoded here. */
async function pixelsOf(png: string): Promise<{ width: number; height: number; data: Uint8ClampedArray } | null> {
  if (typeof createImageBitmap !== "function") return null;
  const bytes = Uint8Array.from(atob(png), (c) => c.charCodeAt(0));
  const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
  try {
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return null;
    ctx.drawImage(bitmap, 0, 0);
    return { width: bitmap.width, height: bitmap.height, data: ctx.getImageData(0, 0, bitmap.width, bitmap.height).data };
  } finally {
    bitmap.close();
  }
}

/**
 * Whether two PNGs (base64) show the same picture: the same text, or the same size and colours. The kernel stores what it gets anew as RGB
 * (`fit_preview`), so a freshly drawn picture never has the stored one's text; the colours compare, alpha does not (the kernel drops it).
 * Pictures that cannot be decoded here (no decoder, a refused picture, text that is no base64) count as different; it never rejects.
 */
export async function samePicture(a: string, b: string): Promise<boolean> {
  if (a === b) return true;
  let pa: Awaited<ReturnType<typeof pixelsOf>>;
  let pb: Awaited<ReturnType<typeof pixelsOf>>;
  try {
    [pa, pb] = await Promise.all([pixelsOf(a), pixelsOf(b)]);
  } catch {
    return false;
  }
  if (pa === null || pb === null || pa.width !== pb.width || pa.height !== pb.height || pa.data.length !== pb.data.length) return false;
  for (let i = 0; i < pa.data.length; i += 4) {
    if (pa.data[i] !== pb.data[i] || pa.data[i + 1] !== pb.data[i + 1] || pa.data[i + 2] !== pb.data[i + 2]) return false;
  }
  return true;
}
