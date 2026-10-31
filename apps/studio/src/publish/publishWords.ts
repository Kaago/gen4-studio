import { formatBytes } from "../api";
import type { BaseStatus, PublishPhase, PublishedFile } from "../api";

/**
 * The words of „Veröffentlichen“ (4.5): the patch's file name, the checks of version and file name as the kernel makes them
 * (`g4_kernel::patch::{check_version, publish::file_name}`, same sentences), the lines of the result and the subjects of „Stand sichern“.
 */

/** `patch::release::BAD_VERSION`, word for word. */
export const BAD_VERSION = 'Die Version darf nicht leer sein, höchstens 32 Zeichen haben und weder Leerzeichen noch / \\ : * ? " < > | enthalten.';
/** `patch::release::VERSION_TAKEN`. */
export const VERSION_TAKEN = "Diese Version gibt es schon.";
/** `patch::publish::BAD_FILE`. */
export const BAD_FILE = 'Der Dateiname darf nicht leer sein und weder / \\ : * ? " < > | noch Steuerzeichen enthalten.';
/** `patch::publish::NO_ROM_CONTENT`: what a release hands on. */
export const NO_ROM_CONTENT = "Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.";
/** Why „Veröffentlichen“ waits in a project without a base mark. */
export const NO_BASE = "Erst eine Basis-ROM zuordnen.";

/** `patch::release::VERSION_MAX`, in characters. */
const VERSION_MAX = 32;
const XDELTA = ".xdelta";
/** The characters a file name cannot have (the kernel's list for version and file name). */
const FORBIDDEN = /[/\\:*?"<>|]/u;
const CONTROL = /\p{Cc}/u;
const SPACE = /\s/u;

/** What runs in each phase of a release. */
export const PHASE_TEXT: Record<PublishPhase, string> = {
  build: "Speichern und bauen …",
  encode: "Patch kodieren …",
  verify: "Selbstprüfung …",
  write: "Dateien schreiben …",
  record: "Veröffentlichung eintragen …",
};

/** `<Projekt>-v<Version>.xdelta`, as the kernel's `release_file_name` names it. */
export function fileNameFor(stem: string, version: string): string {
  return `${stem}-v${version}${XDELTA}`;
}

/** `BAD_VERSION` for an empty or too long version or one with white space, a control character or `/ \ : * ? " < > |`; then `VERSION_TAKEN` for one of `taken`; else null. */
export function versionProblem(version: string, taken: readonly string[]): string | null {
  if (version === "" || [...version].length > VERSION_MAX || SPACE.test(version) || CONTROL.test(version) || FORBIDDEN.test(version)) return BAD_VERSION;
  if (taken.includes(version)) return VERSION_TAKEN;
  return null;
}

/** `BAD_FILE` for a name that is empty (also before `.xdelta`, after cutting spaces and dots) or has a control character or `/ \ : * ? " < > |`; else null. */
export function fileProblem(file: string): string | null {
  const stem = file.endsWith(XDELTA) ? file.slice(0, -XDELTA.length) : file;
  if (stem.replace(/^[ .]+|[ .]+$/gu, "") === "" || CONTROL.test(file) || FORBIDDEN.test(file)) return BAD_FILE;
  return null;
}

/** The name with `.xdelta`, added when missing (as the kernel adds it). */
export function withXdelta(file: string): string {
  return file.endsWith(XDELTA) ? file : `${file}${XDELTA}`;
}

/** „3 Dateien, 5,1 KiB“: how many files a release wrote and their size together. */
export function filesLine(files: readonly PublishedFile[]): string {
  const size = files.reduce((sum, f) => sum + f.size, 0);
  return `${files.length} ${files.length === 1 ? "Datei" : "Dateien"}, ${formatBytes(size)}`;
}

/** „31.10.2026“: the day of an RFC 3339 time, in local time. */
export function dateText(rfc3339: string): string {
  return new Date(rfc3339).toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" });
}

const two = (n: number) => String(n).padStart(2, "0");

/** „Stand 2026-10-31 14:05“: the subject of „Stand sichern“ at `now`, in local time. */
export function commitSubjectNow(now: Date): string {
  return `Stand ${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())} ${two(now.getHours())}:${two(now.getMinutes())}`;
}

/** „Veröffentlicht v0.1.0“: the subject of „Stand sichern“ after a release. */
export function publishedSubject(version: string): string {
  return `Veröffentlicht v${version}`;
}

/**
 * Why „Veröffentlichen“ waits: no found base ROM (`unassigned` → „Erst eine Basis-ROM zuordnen.“, else the status' `reason`), a broken
 * releases list (its sentence); null when it can open. A status not read yet locks nothing: the release itself says why it is refused.
 */
export function publishLocked(status: BaseStatus | null, releasesError: string | null = null): string | null {
  if (status !== null && status.state !== "found") return status.state === "unassigned" ? NO_BASE : status.reason;
  return releasesError;
}
