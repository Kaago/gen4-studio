import { errorText } from "../api";
import { openPath } from "../platform/dialogs";

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
