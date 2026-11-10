import { errorText, projectPreviewSet } from "../api";
import type { PreviewDto } from "../api";
import { t } from "../i18n";
import { openPath } from "../platform/dialogs";
import { toast } from "../ui/frame/toasts";

/** The catalog key of the sentence for each code a refused preview picture carries in its message (phase 4.0 design, §3.5). */
const SENTENCES = { "preview.format": "format", "preview.too_large": "tooLarge", "preview.unreadable": "unreadable" } as const;

const CODE = /\bpreview\.(format|too_large|unreadable)\b/;

/**
 * The bridge's and the path policy's own refusals of a path or of arguments (`bridge/roots.rs`, `commands/policy.rs`, `dispatch.rs`, in
 * the language the app speaks: English or German) and the transport's („Brücke nicht erreichbar“, `platform/transport.ts`, in the
 * language shown): they say what happened already and keep their text.
 */
const OWN_WORDS = [
  // English (`locales/en.yml` of the studio crate)
  /^(Path is outside the allowed folders|Path is not valid text|Invalid arguments for|Path not readable|Folder not readable|Not a file:|Not a folder:|No file name:)/,
  /^(Originals (\(_DSPRE_contents\) )?are locked|Save games \(_saves\) are locked|Only the open project:|Bridge|No Tauri)/,
  // German (`locales/de.yml`)
  /^(Pfad liegt außerhalb|Pfad ist kein gültiger Text|Ungültige Argumente|Pfad nicht lesbar|Ordner nicht lesbar|Keine Datei:|Kein Ordner:|Kein Dateiname:)/,
  /^(Originale (\(_DSPRE_contents\) )?sind gesperrt|Spielstände \(_saves\) sind gesperrt|Nur das offene Projekt:|Brücke|Kein Tauri)/,
];

/**
 * The toast text of a failed preview write: the sentence of its code, the bridge's own refusal, the sentence of an `Error` the app throws
 * itself (no map at the chosen cell, a lost WebGL context), else a sentence that says what to check.
 */
export function previewErrorText(e: unknown): string {
  const text = errorText(e);
  const code = CODE.exec(text)?.[0];
  const key = code === undefined ? undefined : (SENTENCES as Readonly<Record<string, (typeof SENTENCES)[keyof typeof SENTENCES]>>)[code];
  if (key !== undefined) return t(`project:preview.${key}`);
  if (e instanceof Error) return e.message;
  // Any other failure of a write: the kernel's or the system's own text is English and names no way out („Permission denied (os error 13)“).
  return OWN_WORDS.some((words) => words.test(text)) ? text : t("project:preview.other");
}

/** Asks for a picture file (PNG or JPEG) for the preview; null when cancelled. In the browser the path is typed into the page. */
export async function choosePreviewFile(): Promise<string | null> {
  return openPath({ directory: false, title: t("project:preview.choose"), filters: [{ name: t("project:preview.filter"), extensions: ["png", "jpg", "jpeg"] }] });
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
    toast.info(file === null ? t("project:preview.removed") : t("project:preview.set"));
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
