import i18next from "i18next";
import { formatBytes } from "../api";
import type { BaseStatus, PublishPhase, PublishedFile } from "../api";
import { t } from "../i18n";

/**
 * The words of „Veröffentlichen“ (4.5): the patch's file name, the checks of version and file name as the kernel makes them
 * (`g4_kernel::patch::{check_version, publish::file_name}`, same sentences), the lines of the result and the subjects of „Stand sichern“.
 */

/**
 * The sentences below are texts of the `publish` catalog, read when asked (never at module scope: the language can change while the app runs).
 * `badVersion`, `versionTaken`, `badFile` and `noRomContent` are the kernel's sentences, word for word in the language that is shown.
 */

/** `patch::release::BAD_VERSION`, word for word. */
export const badVersion = (): string => t("publish:publishWords.kernel.badVersion");
/** `patch::release::VERSION_TAKEN`. */
export const versionTaken = (): string => t("publish:publishWords.kernel.versionTaken");
/** `patch::publish::BAD_FILE`. */
export const badFile = (): string => t("publish:publishWords.kernel.badFile");
/** `patch::publish::NO_ROM_CONTENT`: what a release hands on. */
export const noRomContent = (): string => t("publish:publishWords.kernel.noRomContent");
/** Why „Veröffentlichen“ waits in a project without a base mark. */
export const noBase = (): string => t("publish:publishWords.noBase");

/** `patch::release::VERSION_MAX`, in characters. */
const VERSION_MAX = 32;
const XDELTA = ".xdelta";
/** The characters a file name cannot have (the kernel's list for version and file name). */
const FORBIDDEN = /[/\\:*?"<>|]/u;
const CONTROL = /\p{Cc}/u;
const SPACE = /\s/u;

/** What runs in each phase of a release. */
export const phaseText = (phase: PublishPhase): string => t(`publish:publishWords.phase.${phase}`);

/** `<Projekt>-v<Version>.xdelta`, as the kernel's `release_file_name` names it. */
export function fileNameFor(stem: string, version: string): string {
  return `${stem}-v${version}${XDELTA}`;
}

/** `badVersion` for an empty or too long version or one with white space, a control character or `/ \ : * ? " < > |`; then `versionTaken` for one of `taken`; else null. */
export function versionProblem(version: string, taken: readonly string[]): string | null {
  if (version === "" || [...version].length > VERSION_MAX || SPACE.test(version) || CONTROL.test(version) || FORBIDDEN.test(version)) return badVersion();
  if (taken.includes(version)) return versionTaken();
  return null;
}

/** `badFile` for a name that is empty (also before `.xdelta`, after cutting spaces and dots) or has a control character or `/ \ : * ? " < > |`; else null. */
export function fileProblem(file: string): string | null {
  const stem = file.endsWith(XDELTA) ? file.slice(0, -XDELTA.length) : file;
  if (stem.replace(/^[ .]+|[ .]+$/gu, "") === "" || CONTROL.test(file) || FORBIDDEN.test(file)) return badFile();
  return null;
}

/** The name with `.xdelta`, added when missing (as the kernel adds it). */
export function withXdelta(file: string): string {
  return file.endsWith(XDELTA) ? file : `${file}${XDELTA}`;
}

/** „3 Dateien, 5,1 KiB“: how many files a release wrote and their size together. */
export function filesLine(files: readonly PublishedFile[]): string {
  const size = files.reduce((sum, f) => sum + f.size, 0);
  return t("publish:publishWords.filesLine", { count: files.length, size: formatBytes(size) });
}

/** „31.10.2026“ (German), „Oct 31, 2026“ (English): the day of an RFC 3339 time, in local time. */
export function dateText(rfc3339: string): string {
  const date = new Date(rfc3339);
  if (i18next.language === "de") return date.toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" });
  return date.toLocaleDateString("en-US", { day: "numeric", month: "short", year: "numeric" });
}

const two = (n: number) => String(n).padStart(2, "0");

/** „Stand 2026-10-31 14:05“ („Snapshot 2026-10-31 14:05“ in English): the subject of „Stand sichern“ at `now`, in local time. */
export function commitSubjectNow(now: Date): string {
  const when = `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())} ${two(now.getHours())}:${two(now.getMinutes())}`;
  return t("publish:publishWords.subject.now", { when });
}

/** „Veröffentlicht v0.1.0“ („Published v0.1.0“): the subject of „Stand sichern“ after a release. */
export function publishedSubject(version: string): string {
  return t("publish:publishWords.subject.published", { version });
}

/**
 * Why „Veröffentlichen“ waits: no found base ROM (`unassigned` → `noBase`, else the status' `reason`), a broken
 * releases list (its sentence); null when it can open. A status not read yet locks nothing: the release itself says why it is refused.
 */
export function publishLocked(status: BaseStatus | null, releasesError: string | null = null): string | null {
  if (status !== null && status.state !== "found") return status.state === "unassigned" ? noBase() : status.reason;
  return releasesError;
}
