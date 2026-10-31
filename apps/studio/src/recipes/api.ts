import { invoke } from "../platform/transport";
import type { AssetId, AssetKind } from "../api";

/**
 * The recipes (phase 3d.2), mirroring `apps/studio/src-tauri/src/commands/recipes.rs` and `g4_kernel::recipes`. `api.ts`
 * re-exports everything. Field names are the Rust ones (snake_case). A plan writes nothing and answers a draft the scene and
 * the wizards show dashed; an application is one undo step.
 */

export type RecipeId = "create_area" | "create_room" | "extend_area" | "empty_region";

/** A recipe as `recipeList` names it: `irreversible` when applying it always does something undo does not take back. */
export interface RecipeInfo {
  id: RecipeId;
  title: string;
  irreversible: boolean;
}

/** A cell the recipe fills with a copy of `source_map`; `matrix` null is a matrix the recipe creates. */
export interface DraftCell {
  matrix: number | null;
  x: number;
  y: number;
  source_map: number;
}

/** A tile the recipe changes (a door tile). */
export interface DraftTile {
  matrix: number | null;
  x: number;
  z: number;
}

/** An event the recipe adds or changes; a warp's `target` names where it leads. */
export interface DraftEvent {
  kind: "object" | "warp" | "coord" | "bg";
  matrix: number | null;
  x: number;
  z: number;
  look: number | null;
  target: { header: number; warp: number | null } | null;
}

/** What applying a recipe would do, found without writing: `notes` are German sentences for the wizard, `irreversible` what
 * undo would not take back („Karte 0187 wird kopiert“). `archive` null is a text archive the recipe creates. */
export interface DraftDto {
  cells: DraftCell[];
  tiles: DraftTile[];
  events: DraftEvent[];
  messages: { archive: number | null; text: string }[];
  notes: string[];
  irreversible: string[];
}

/** A recipe that makes sense next, as a button: `reason` says when a locked one comes („Kommt mit 3d.5“). */
export interface NextStep {
  id: string;
  label: string;
  available: boolean;
  reason: string | null;
}

/** What an applied recipe did: its report, the members it created first (they stay after undo; a new header is `HeaderTable`
 * with its id), its undo step (null only when it changed nothing), every asset to load again and the next steps. */
export interface RecipeOutcome<R = unknown> {
  report: R;
  created: { kind: AssetKind; index: number }[];
  change: number | null;
  changed: AssetId[];
  next: NextStep[];
}

export const recipeList = () => invoke<RecipeInfo[]>("recipe_list");

/** What applying `recipe` with `input` would do to the open project (unsaved edits included); edits nothing. Rejects with the
 * refusal the application would give (German sentence). */
export const recipePlan = (recipe: RecipeId, input: unknown) => invoke<DraftDto>("recipe_plan", { recipe, input });

/** Applies `recipe` with `input`: new members first, then one undo step. A failure after the first member is a `"partial"`
 * error naming what was created. */
export const recipeApply = <R = unknown>(recipe: RecipeId, input: unknown) =>
  invoke<RecipeOutcome<R>>("recipe_apply", { recipe, input });

/** What the scene of `matrix` draws of `draft` (cells, tiles and events of that matrix). */
export function draftCount(draft: DraftDto | null, matrix: number): number {
  if (!draft) return 0;
  const on = (item: { matrix: number | null }) => item.matrix === matrix;
  return draft.cells.filter(on).length + draft.tiles.filter(on).length + draft.events.filter(on).length;
}
