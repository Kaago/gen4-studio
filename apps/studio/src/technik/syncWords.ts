import type { SyncProblem, SyncProgress } from "../api";
import { NBSP, count } from "../words";

/** The words of the sync with DSPRE (Technik › Textprojekt, the app's messages): each count with its singular and plural. */


/** The message after DSPRE's changes were taken over. */
export const adoptedText = (n: number) => count(n, "Änderung aus DSPRE übernommen", "Änderungen aus DSPRE übernommen");

/** The message after changed project files (a `git pull`, a hand edit) were taken over. */
export const appliedText = (n: number) => count(n, "geänderte Projektdatei übernommen", "geänderte Projektdateien übernommen");

/** The warning that entries changed on both sides. */
export const conflictsText = (n: number) => count(n, "Konflikt zwischen Studio und DSPRE", "Konflikte zwischen Studio und DSPRE");

/** The error that project files do not read. */
export const brokenText = (n: number) => count(n, "Projektdatei ist fehlerhaft", "Projektdateien sind fehlerhaft");

/** The conflicts in the list of Technik. */
export const conflictCountText = (n: number) => count(n, "Konflikt", "Konflikte");

/** The broken files in the list of Technik (after the conflicts). */
export const brokenCountText = (n: number) => count(n, "fehlerhafte Datei", "fehlerhafte Dateien");

/** Below a diff cut at its 40 lines. */
export const moreText = (n: number) => `und ${count(n, "weitere Abweichung", "weitere Abweichungen")}`;

/** The line of a sync under way. */
export const syncProgressText = ({ done, total }: SyncProgress) => `Gleiche mit DSPRE ab … ${done} von ${total}${NBSP}Dateien`;

/** Below the line of a full sync (the first one hashes every file). */
export const FIRST_SYNC_NOTE = "Das erste Mal dauert länger; danach geht es schnell.";

/** The text project's check code in front of a problem's sentence (`textproject.schema: `): a code for tools, no part of the sentence. */
const CHECK_CODE = /^textproject(?:\.[a-z0-9_-]+)+: /;

/** A problem of a broken file with its place: `{file}:{line}:{col}: {sentence}`, without the kernel's check code. */
export const problemLine = (p: SyncProblem) => `${p.file}:${p.line}:${p.col}: ${p.message.replace(CHECK_CODE, "")}`;

/** What a build with open conflicts says (the backend's `conflicts` refusal says the same). */
export const BUILD_CONFLICTS = "Erst die Konflikte in Technik › Textprojekt lösen.";

/** The key of the one message about open conflicts: the sync's warning, and a build's refusal in its place. */
export const CONFLICTS_TOAST = "sync-conflicts";
