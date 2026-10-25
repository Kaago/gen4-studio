import { invoke } from "../platform/transport";
import type { AssetId } from "../api";

/**
 * The empty-region template on the open project, mirroring `apps/studio/src-tauri/src/commands/templates.rs` and
 * `g4_kernel::templates`. `api.ts` re-exports everything. Field names are the Rust ones (snake_case). A new project gets the
 * template through `createProject(…, "empty_region", …)`; these two work on a project that is open.
 */

/** What the recipe would do to the world as the store has it now (unsaved edits included). */
export interface RegionPlan {
  /** The start town's header. */
  start_header: number;
  /** The town's in-game location name, for example "Zweiblattdorf"; empty when the text bank does not name it. */
  town_name: string;
  /** `[x, y]` of the town's cells in matrix 0, row by row. */
  town_cells: [number, number][];
  /** Map of the first town cell: the source of the border map. */
  town_map: number;
  /** Altitude of the first town cell, which every border cell gets. */
  town_altitude: number;
  town_event_file: number;
  town_script_file: number;
  /** The town and the headers reached from it through warps without entering another header of matrix 0, ascending. */
  kept: number[];
  /** Headers that get event file 0 and the empty level script, ascending. */
  emptied: number[];
  /** Cells of matrix 0 outside the town; they get the border map. */
  border_cells: number;
  /** The level script every emptied header gets. */
  empty_level_script: number;
  /** `[event file, warp index]` of the warps of kept event files that lead into a header that is not kept; they are removed. */
  dropped_warps: [number, number][];
  /** Figures (object events) of the town's event file, removed. */
  removed_objects: number;
  /** Triggers (coord events) of the town's event file, removed. */
  removed_triggers: number;
  /** Species the start script gives (the recipe's starter). */
  starter_species: number;
  /** Level of that Pokémon. */
  starter_level: number;
  /** The world already looks like the recipe's result; `applyEmptyRegion` refuses then. */
  already_empty: boolean;
}

/** What `applyEmptyRegion` did. */
export interface RegionReport {
  /** The plan it carried out. */
  plan: RegionPlan;
  /** The new map of every cell outside the town (written at once, not undoable). */
  border_map: number;
  /** The new level script of the town (written at once, not undoable). */
  level_script: number;
  /** `[script file, number]` of the start script appended to the town's script file (written at once, not undoable). */
  start_script: [number, number];
  /** The store step (one undo); null only when it changed nothing. */
  change: number | null;
  /** Every asset it changed, in kind and index order: load these again. */
  changed: AssetId[];
}

/**
 * What the empty region would do to the open project; edits nothing. Rejects for Diamond/Pearl, for a start town the project does
 * not have and for a project without an empty level script.
 */
export const planEmptyRegion = () => invoke<RegionPlan>("plan_empty_region");

/**
 * Applies the empty region to the open project. Rejects ("save first") with unsaved changes, without adopted scripts
 * ("scripts are not adopted") and on a world that is empty already ("the world is already empty"). The new border map, level
 * script and start script are written at once; the world edits are one undoable step. Takes seconds.
 */
export const applyEmptyRegion = () => invoke<RegionReport>("apply_empty_region");
