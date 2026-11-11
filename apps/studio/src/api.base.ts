import type { Effective } from "./i18n";
import { t } from "./i18n";
import { inTauri, invoke } from "./platform/transport";
import type { RomInfo, Template } from "./project/api.start";

/**
 * The base ROM of the open project (Technik › Veröffentlichen), mirroring `apps/studio/src-tauri/src/commands/base.rs` and
 * `g4_kernel::{start, base}`. `api.ts` re-exports everything. Field names are the Rust ones (snake_case). A broken mark is a state
 * (`unreadable`), never an error. Releases against the base ROM are `api.publish.ts`.
 */

/** The base mark `.g4/base.json` (version 2). `template` only for projects made from a ROM, `assigned` only for DSPRE folders. */
export interface BaseMarkDto {
  version: number;
  origin: "extracted" | "dspre";
  /** SHA-1 of the whole base ROM, 40 lowercase hex digits. */
  sha1: string;
  game_code: string;
  title: string;
  revision: number;
  size: number;
  template?: Template;
  created: string;
  assigned?: string;
  studio_version: string;
  secure_area_crc: number;
  secure_area_sha1: string;
}

/** Where the base ROM was found: `--base-rom`, the remembered entry, the `.nds` next to the folder, `G4_BASE_ROM_DIR`. */
export type BaseSource = "explicit" | "known" | "sibling" | "dir";

/**
 * The base ROM of the open project. `label` names it („Platin DE (Testprojekt)“); `reason` is a sentence in the backend's language
 * (`set_locale`) ready to show; `patch_name` is `<Projekt>-<Spielcode>`, the name the agents' `create_patch` writes under `build/` (the
 * kernel's `patch::patch_name`).
 */
export type BaseStatus =
  | { state: "unassigned"; suggestion: string | null }
  | { state: "missing"; mark: BaseMarkDto; label: string; reason: string }
  | { state: "found"; mark: BaseMarkDto; label: string; path: string; source: BaseSource; patch_name: string }
  | { state: "unreadable"; reason: string };

export interface BaseWarning {
  kind: "title_differs" | "revision_differs" | "unknown_dump" | "arm7_differs" | "banner_differs";
  /** A sentence in the backend's language (`set_locale`) for the user. */
  message: string;
}

/** What `checkBase` found; `refused` (only another game code) blocks the assignment, then `warnings` is empty. */
export interface BaseCheck {
  info: RomInfo;
  warnings: BaseWarning[];
  refused: string | null;
}

/** The base ROM of the open project; may hash a ROM to find it. */
export const baseStatus = () => invoke<BaseStatus>("base_status");
/** Checks the `.nds` at `nds` as the base ROM of the open project; writes nothing. */
export const checkBase = (nds: string) => invoke<BaseCheck>("check_base", { nds });
/** Writes the base mark for the `.nds` at `nds`, remembers the ROM and answers the new status. */
export const assignBase = (nds: string) => invoke<BaseStatus>("assign_base", { nds });
/** Remembers where the base ROM lies now („Basis-ROM suchen…“); another ROM is refused and the entry stays. */
export const registerBaseRom = (path: string) => invoke<BaseStatus>("register_base_rom", { path });

const UNITS = ["KiB", "MiB", "GiB"] as const;

/** A size for people: „397 Byte“, „4,7 KiB“, „1,2 MiB“ (binary units, one decimal with the decimal mark of the language shown). */
export function formatBytes(n: number): string {
  if (n < 1024) return t("common:bytes.bytes", { count: n });
  let value = n / 1024;
  let unit = 0;
  // Rounded to one decimal, 1023.95 KiB would read „1024,0 KiB“: move on to the next unit.
  while (Math.round(value * 10) / 10 >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return t("common:bytes.unit", { value: value.toFixed(1).replace(".", t("common:bytes.decimalMark")), unit: UNITS[unit] });
}

/**
 * Tells the backend the language of its own texts (`set_locale`, `de` or `en`). Inside Tauri the backend is the app's process; in the
 * browser during development it is the dev bridge, a process of its own that reads `G4_LANG` at its start and has no other way to learn
 * a switch. Elsewhere (the tests, a build without a backend) there is nothing to tell: it resolves at once.
 */
export async function setLocale(lang: Effective): Promise<void> {
  if (!inTauri() && import.meta.env.MODE !== "development") return;
  await invoke<void>("set_locale", { lang });
}

/**
 * The language of the operating system as the backend reads it (`system_language`, for example „de-DE“), for the choice „System“: inside
 * Tauri `navigator.language` may report the bundle's localizations rather than the OS language. `null` outside Tauri (the browser's
 * navigator is the system's there) and when the OS reports none.
 */
export async function systemLanguage(): Promise<string | null> {
  if (!inTauri()) return null;
  return invoke<string | null>("system_language");
}
