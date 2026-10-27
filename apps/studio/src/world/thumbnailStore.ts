import { getMapModelGlb } from "../api";
import { toolStatus } from "../app/toolStatus";
import { createTopDownRenderer } from "./thumbnailRenderer";
import { ThumbnailCache } from "./thumbnails";

/**
 * The models of the pictures come from apicula: the first one that comes back says it works. Only while nothing is
 * known: the backend also hands out cached models while apicula is missing, so a model after a missing tool proves nothing.
 */
export async function mapModelGlb(map: number, area: number): Promise<ArrayBuffer> {
  const bytes = await getMapModelGlb(map, area);
  if (toolStatus.get().apicula === "unbekannt") toolStatus.set({ apicula: "bereit" });
  return bytes;
}

/** The app-wide map pictures. Importing this touches no WebGL: the renderer is created on the first picture. */
export const thumbnails = new ThumbnailCache({ glb: mapModelGlb }, createTopDownRenderer());

// A picture that failed for a missing conversion tool: Technik says apicula is missing.
thumbnails.onToolMissing(() => toolStatus.set({ apicula: "fehlt" }));
