import { errorText, projectPreviewSet } from "../api";
import type { PreviewDto } from "../api";
import { openPath } from "../platform/dialogs";
import { toast } from "../ui/frame/toasts";

/** The sentence for each code a refused preview picture carries in its message (phase 4.0 design, §3.5). */
const SENTENCES: Record<string, string> = {
  "preview.format": "Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.",
  "preview.too_large": "Das Bild ist zu groß (höchstens 32 MB und 8192 × 8192 Pixel). Wähle ein kleineres Bild.",
  "preview.unreadable": "Das Bild lässt sich nicht lesen. Wähle eine andere Datei.",
};

const CODE = /\bpreview\.(format|too_large|unreadable)\b/;

/** The toast text of a failed `project_preview_set`: the sentence of its code, else the error's own text. */
export function previewErrorText(e: unknown): string {
  const text = errorText(e);
  const code = CODE.exec(text)?.[0];
  return (code !== undefined && SENTENCES[code]) || text;
}

/** Asks for a picture file (PNG or JPEG) for the preview; null when cancelled. In the browser the path is typed into the page. */
export async function choosePreviewFile(): Promise<string | null> {
  return openPath({ directory: false, title: "Vorschaubild wählen", filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }] });
}

/**
 * „Vorschaubild wählen…“ (`choose`: asks for a file, then writes it) or removing the own picture (`remove`), for the project at `path`
 * (Start) or the open project without one (Technik). Says so in a toast: „Vorschaubild gesetzt“ or „Vorschaubild entfernt“, a refused
 * picture with the sentence of its code, a failing dialog with its text. Answers the preview after the write; null when cancelled or
 * failed (never rejects).
 */
export async function changePreview(how: "choose" | "remove", path?: string): Promise<PreviewDto | null> {
  const where = path === undefined ? {} : { path };
  try {
    const file = how === "choose" ? await choosePreviewFile() : null;
    if (how === "choose" && file === null) return null;
    const preview = await projectPreviewSet(file === null ? where : { ...where, file });
    toast.info(file === null ? "Vorschaubild entfernt" : "Vorschaubild gesetzt");
    return preview;
  } catch (e) {
    toast.error(previewErrorText(e));
    return null;
  }
}

/**
 * Says that the open project's preview was written elsewhere than in Technik (from the scene, or the standard picture in the background),
 * with the answer of the write: Technik's section, if it shows, takes it at once. Start reads its cards again each time it shows anyway.
 */
export const previewWritten = (() => {
  const listeners = new Set<(preview: PreviewDto) => void>();
  return {
    subscribe(listener: (preview: PreviewDto) => void): () => void {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    emit(preview: PreviewDto): void {
      for (const listener of [...listeners]) listener(preview);
    },
  };
})();
