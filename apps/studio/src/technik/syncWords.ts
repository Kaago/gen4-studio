import type { SyncProblem, SyncProgress } from "../api";

/** The words of the sync with DSPRE (Technik › Textprojekt, the app's messages): each count with its singular and plural. */

const count = (n: number, one: string, many: string) => (n === 1 ? one : `${n} ${many}`);

/** The message after DSPRE's changes were taken over. */
export const adoptedText = (n: number) => count(n, "1 Änderung aus DSPRE übernommen", "Änderungen aus DSPRE übernommen");

/** The message after changed project files (a `git pull`, a hand edit) were taken over. */
export const appliedText = (n: number) => count(n, "1 geänderte Projektdatei übernommen", "geänderte Projektdateien übernommen");

/** The warning that entries changed on both sides. */
export const conflictsText = (n: number) => count(n, "1 Konflikt zwischen Studio und DSPRE", "Konflikte zwischen Studio und DSPRE");

/** The error that project files do not read. */
export const brokenText = (n: number) => count(n, "1 Projektdatei ist fehlerhaft", "Projektdateien sind fehlerhaft");

/** The conflicts in the list of Technik. */
export const conflictCountText = (n: number) => count(n, "1 Konflikt", "Konflikte");

/** Below a diff cut at its 40 lines. */
export const moreText = (n: number) => `und ${count(n, "1 weitere Abweichung", "weitere Abweichungen")}`;

/** The line of a sync under way. */
export const syncProgressText = ({ done, total }: SyncProgress) => `Gleiche mit DSPRE ab … ${done} von ${total} Dateien`;

/** Below the line of a full sync (the first one hashes every file). */
export const FIRST_SYNC_NOTE = "Das erste Mal dauert länger; danach geht es schnell.";

/** A problem of a broken file with its place: `{file}:{line}:{col}: {sentence}`. */
export const problemLine = (p: SyncProblem) => `${p.file}:${p.line}:${p.col}: ${p.message}`;

/** What a build with open conflicts says (the backend's `conflicts` refusal says the same). */
export const BUILD_CONFLICTS = "Erst die Konflikte in Technik › Textprojekt lösen.";
