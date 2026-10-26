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

/** The bridge's own refusals of a path or of arguments (`bridge/roots.rs`, `dispatch.rs`): German already, they keep their text. */
const GERMAN = /^(Pfad liegt außerhalb|Pfad ist kein gültiger Text|Ungültige Argumente)/;

/** Any other failure of a write: the kernel's or the system's own text is English and names no way out („Permission denied (os error 13)“). */
const OTHER = "Das Vorschaubild ließ sich nicht ändern. Prüfe, ob der Projektordner noch da ist und du darin schreiben darfst.";

/** The toast text of a failed `project_preview_set`: the sentence of its code, the bridge's German refusal, else a German sentence. */
export function previewErrorText(e: unknown): string {
  const text = errorText(e);
  const code = CODE.exec(text)?.[0];
  if (code !== undefined && SENTENCES[code]) return SENTENCES[code];
  return GERMAN.test(text) ? text : OTHER;
}

/** Asks for a picture file (PNG or JPEG) for the preview; null when cancelled. In the browser the path is typed into the page. */
export async function choosePreviewFile(): Promise<string | null> {
  return openPath({ directory: false, title: "Vorschaubild wählen", filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }] });
}

/**
 * „Vorschaubild wählen…“ (`choose`: asks for a file, then writes it) or removing the own picture (`remove`), for the project at `path`
 * (Start) or the open project without one (Technik). Says so in a toast: „Vorschaubild gesetzt“ or „Vorschaubild entfernt“, a failed
 * write as `previewErrorText` says it, a failing dialog with its text. Answers the preview after the write; null when cancelled or
 * failed (never rejects).
 */
export async function changePreview(how: "choose" | "remove", path?: string): Promise<PreviewDto | null> {
  const where = path === undefined ? {} : { path };
  let file: string | null = null;
  if (how === "choose") {
    try {
      file = await choosePreviewFile();
    } catch (e) {
      toast.error(errorText(e));
      return null;
    }
    if (file === null) return null;
  }
  try {
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
 * with the answer of the write and the folder it was written to (`path`; none for a write without one, which goes to the open project):
 * Technik's section, if it shows, takes it at once; Start, if it shows, reads that folder's card again. Start reads its cards again each
 * time it shows anyway.
 */
export const previewWritten = (() => {
  type Listener = (preview: PreviewDto, path?: string) => void;
  const listeners = new Set<Listener>();
  return {
    subscribe(listener: Listener): () => void {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    emit(preview: PreviewDto, path?: string): void {
      for (const listener of [...listeners]) listener(preview, path);
    },
  };
})();
