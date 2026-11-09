import type { SyncProblem, SyncProgress } from "../api";
import { t } from "../i18n";

/** The words of the sync with DSPRE (Technik › Textprojekt, the app's messages): each count with its singular and plural, from the `technik` catalog. */

/** The message after DSPRE's changes were taken over. */
export const adoptedText = (n: number) => t("technik:syncWords.adoptedCount", { count: n });

/** The message after changed project files (a `git pull`, a hand edit) were taken over. */
export const appliedText = (n: number) => t("technik:syncWords.appliedCount", { count: n });

/** The warning that entries changed on both sides. */
export const conflictsText = (n: number) => t("technik:syncWords.conflictsCount", { count: n });

/** The error that project files do not read. */
export const brokenText = (n: number) => t("technik:syncWords.brokenCount", { count: n });

/** The conflicts in the list of Technik. */
export const conflictCountText = (n: number) => t("technik:syncWords.conflictListCount", { count: n });

/** The broken files in the list of Technik (after the conflicts). */
export const brokenCountText = (n: number) => t("technik:syncWords.brokenListCount", { count: n });

/** Below a diff cut at its 40 lines. */
export const moreText = (n: number) => t("technik:syncWords.moreCount", { count: n });

/** The line of a sync under way. */
export const syncProgressText = ({ done, total }: SyncProgress) => t("technik:syncWords.progress", { done, total });

/** Below the line of a full sync (the first one hashes every file). */
export const firstSyncNote = () => t("technik:shared.firstRunNote");

/** The text project's check code in front of a problem's sentence (`textproject.schema: `): a code for tools, no part of the sentence. */
const CHECK_CODE = /^textproject(?:\.[a-z0-9_-]+)+: /;

/** A problem of a broken file with its place: `{file}:{line}:{col}: {sentence}`, without the kernel's check code. */
export const problemLine = (p: SyncProblem) => `${p.file}:${p.line}:${p.col}: ${p.message.replace(CHECK_CODE, "")}`;

/** What a build with open conflicts says (the backend's `conflicts` refusal says the same), in the language that is shown. */
export const buildConflictsText = () => t("technik:syncWords.buildConflicts");

/**
 * @deprecated German only, kept while `app/useProjectSession.ts` (and `App.test.tsx`) import the name `BUILD_CONFLICTS` from here: a constant
 * cannot follow the language. They move to `buildConflictsText()`, then this goes.
 */
export const BUILD_CONFLICTS = "Erst die Konflikte in Technik › Textprojekt lösen.";

/** The key of the one message about open conflicts: the sync's warning, and a build's refusal in its place. */
export const CONFLICTS_TOAST = "sync-conflicts";
