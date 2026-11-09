import { resolveScript } from "../../api";
import type { HeaderSummary } from "../../api";
import { t } from "../../i18n";
import workbenchDe from "../../i18n/locales/de/workbench.json";
import { id4, termLabel } from "../terms";

/** Shown instead of the open and new buttons while rotom has not taken the scripts over (the commands would fail). */
export const adoptHint = (): string => t("workbench:scriptNav.adoptHint");

/**
 * The German sentence of `adoptHint` (read from the German catalog, so it has one source), for `WorldTree.tsx` (outside this folder's
 * migration) until it reads `adoptHint()`.
 * @deprecated Use `adoptHint()`: this one stays German in English.
 */
export const ADOPT_HINT: string = workbenchDe.scriptNav.adoptHint;

/**
 * Opens script `n` as the game finds it from `header` (its own script file, or the shared ones): the script
 * panel at the line of `script … #n`. A script that several files define or none is reported through `onError`.
 */
export async function openScriptNumber(header: number, n: number, onOpenScript: (file: number, line?: number) => void, onError: (e: unknown) => void): Promise<void> {
  try {
    const target = await resolveScript(header, n);
    switch (target.kind) {
      case "file":
        onOpenScript(target.file, target.line);
        return;
      case "ambiguous":
        onError(t("workbench:scriptNav.ambiguous", { script: termLabel("script"), n, files: target.candidates.map(id4).join(", ") }));
        return;
      case "unknown":
        onError(t("workbench:scriptNav.unknown", { script: termLabel("script"), n }));
        return;
    }
  } catch (e) {
    onError(e);
  }
}

/**
 * The header an event of `eventFile` belongs to: the selected one when it uses that file, else the first that does
 * (headers can share an event file; the script file is the header's).
 */
export function headerOfEventFile(headers: readonly HeaderSummary[], eventFile: number, selected: number | null): HeaderSummary | undefined {
  const chosen = selected === null ? undefined : headers.find((h) => h.id === selected);
  return chosen?.event_file_id === eventFile ? chosen : headers.find((h) => h.event_file_id === eventFile);
}
