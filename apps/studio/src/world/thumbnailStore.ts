import { getMapModelGlb } from "../api";
import { createTopDownRenderer } from "./thumbnailRenderer";
import { ThumbnailCache } from "./thumbnails";

/** The app-wide map pictures. Importing this touches no WebGL: the renderer is created on the first picture. */
export const thumbnails = new ThumbnailCache({ glb: getMapModelGlb }, createTopDownRenderer());
