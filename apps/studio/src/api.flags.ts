import type { AssetId, NavTargetDto } from "./api";
import { invoke } from "./platform/transport";

/**
 * Flags and variables (3d.1), mirroring `apps/studio/src-tauri/src/commands/flags.rs` and `g4_kernel::flags`. `api.ts` re-exports
 * everything. Field names are the Rust ones (snake_case). `freeFlag` refuses with code `scripts_not_adopted` while the scripts are not
 * adopted (the index cannot see what scripts use) and with `pool_empty` when the general area has no free number; `nameFlag` with
 * `not_text_project` outside a text project, `invalid_id`, `taken` or `refused`. Every refusal carries the kernel's German sentence.
 */

/** Whether a number is a flag or a variable. */
export type FlagKind = "flag" | "var";

/** What a range of numbers is for (`metadata/flag_ranges.yaml`); `outside`: flag 0, numbers past the save, variables below `0x4000`. */
export type FlagArea = "map_local" | "general" | "hidden_items" | "trainer" | "system" | "daily" | "temp" | "object_gfx" | "script_local" | "outside";

/** What a reference does with its flag or variable. */
export type FlagAccess = "read" | "write" | "read_write" | "unknown";

/** How often the index knows a number to be read and written (`read_write` counts for both), and by how many distinct sources. */
export interface FlagUsageCounts {
  read: number;
  write: number;
  unknown: number;
  sources: number;
}

/**
 * One number of the list: its pret name (`name`, the main one; the others in `aliases`), its own name and description in a text project,
 * its area, whether the save holds it (`in_save`: only such a number takes an own name), its usage, whether it is free (the pool), which recipe reserves it (`reserved`, a German word) and at how many places
 * (distinct headers) a map-local number is used (0 for every other area).
 */
export interface FlagEntryDto {
  number: number;
  name: string | null;
  aliases: string[];
  own_id: string | null;
  description: string | null;
  area: FlagArea;
  in_save: boolean;
  usage: FlagUsageCounts;
  free: boolean;
  reserved: string | null;
  places: number;
}

/**
 * `listFlags`: every number of a kind with a name or a use, the pool's size, what scripts use without a number (`counts`), whether the
 * index sees the script sources (`scripts_indexed`: false while the scripts are not adopted; `free` then cannot know what scripts use),
 * whether the project is a text project, its family and how long the list took.
 */
export interface FlagListDto {
  kind: FlagKind;
  entries: FlagEntryDto[];
  pool_free: number;
  pool_total: number;
  counts: { script_local: number; dynamic: number };
  scripts_indexed: boolean;
  text_project: boolean;
  family: string;
  ms: number;
}

/**
 * One reference to a flag or variable: `label` says what of the source refers („Objekt 3 · Sichtbarkeits-Flag“), `source_name` the
 * source („Eventdatei 0394“), `nav` where a click leads, `header` the first header that uses the source (with its name).
 */
export interface FlagRefDto {
  source: AssetId;
  field: string;
  slot: number | null;
  access: FlagAccess;
  label: string;
  source_name: string;
  nav: NavTargetDto;
  header: number | null;
  header_name: string | null;
  /** For a variable: the constant the reference sets or compares („= 3“ behind the label); null for flags and where the other operand is a variable. */
  value: number | null;
}

/** `flagUsage`: one number with its names, area, whether the save holds it, usage and every reference. */
export interface FlagUsageDto {
  kind: FlagKind;
  number: number;
  name: string | null;
  own_id: string | null;
  description: string | null;
  area: FlagArea;
  in_save: boolean;
  usage: FlagUsageCounts;
  reserved: string | null;
  refs: FlagRefDto[];
  ms: number;
}

/** `freeFlag`: the smallest free number, its pret name, how many are free, and that the index sees the scripts (always, else it refuses). */
export interface FreeFlagDto {
  kind: FlagKind;
  number: number;
  name: string | null;
  pool_free: number;
  scripts_indexed: boolean;
}

/** What `nameFlag` wrote: the id, whether it was new, the files (relative to the root), the change on the undo stack, and what failed after. */
export interface NameReport {
  kind: FlagKind;
  number: number;
  id: string;
  created: boolean;
  files: string[];
  change: number | null;
  ms: number;
  warnings: string[];
}

/** An own name (`id`) and/or a description for a number of a text project. */
export interface NameFlagRequest {
  kind: FlagKind;
  number: number;
  id?: string;
  description?: string;
}

/** Builds the reference index on first use (seconds on a large project). */
export const listFlags = (kind: FlagKind) => invoke<FlagListDto>("list_flags", { kind });
/** Where number `number` of `kind` is used. */
export const flagUsage = (kind: FlagKind, number: number) => invoke<FlagUsageDto>("flag_usage", { kind, number });
/** The smallest free number of `kind` (see the codes above). */
export const freeFlag = (kind: FlagKind) => invoke<FreeFlagDto>("free_flag", { kind });
/** Names a number or changes its description as one undo step; only the fields given are sent. */
export const nameFlag = ({ kind, number, id, description }: NameFlagRequest) =>
  invoke<NameReport>("name_flag", { kind, number, ...(id === undefined ? {} : { id }), ...(description === undefined ? {} : { description }) });
