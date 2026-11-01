import { invoke } from "../platform/transport";
import type { AssetId, AssetKind } from "../api";

/**
 * The recipes (phase 3d.2), mirroring `apps/studio/src-tauri/src/commands/recipes.rs` and `g4_kernel::recipes`. `api.ts`
 * re-exports everything. Field names are the Rust ones (snake_case). A plan writes nothing and answers a draft the scene and
 * the wizards show dashed; an application is one undo step.
 */

export type RecipeId = "create_area" | "create_room" | "extend_area" | "empty_region" | "new_figure" | "connect_doors" | "new_trainer_here";

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

/**
 * The input of „Neue Figur“ (`g4_kernel::recipes::new_figure`): the tile (`x`, `z`, absolute tiles of the header's matrix), the
 * look (absent or null: the family's default figure), how the figure moves (`movement_type`) and looks (`dir`), the text it says
 * and the flag that hides it while set (absent, null or 0: always visible; a free one comes from `freeFlag`).
 */
export interface NewFigureInput {
  header: number;
  x: number;
  z: number;
  graphics_id?: number | null;
  movement_type: number;
  dir: number;
  text: string;
  hidden_flag?: number | null;
}

/** What „Neue Figur“ did: object `object` (its index in `event_file`) with `local_id`, script `#slot` in `script_file`, message
 * `message` in text archive `archive`. */
export interface NewFigureReport {
  event_file: number;
  object: number;
  local_id: number;
  script_file: number;
  slot: number;
  archive: number;
  message: number;
}

/** A tile of a header's matrix, absolute. */
export interface RecipeTile {
  x: number;
  z: number;
}

/** One end of „Tür verbinden“: a door that is there (`warp`, its index in the header's event file) or a tile that gets a new door;
 * exactly one of the two (the kernel refuses unknown keys). */
export type DoorEnd = { header: number; warp: number; tile?: null } | { header: number; tile: RecipeTile; warp?: null };

/** The input of „Tür verbinden“ (`g4_kernel::recipes::connect_doors`): A leads to B; with `back` (default true) B leads to A as
 * well; with `door_tiles` (default true) the tile of a new door becomes a door tile. */
export interface ConnectDoorsInput {
  a: DoorEnd;
  b: DoorEnd;
  back?: boolean;
  door_tiles?: boolean;
}

/** One end as „Tür verbinden“ left it: warp `warp` of `event_file`, new (`created`) or existing, and the door tile it got. */
export interface DoorReport {
  header: number;
  event_file: number;
  warp: number;
  created: boolean;
  door_tile: { map: number; x: number; y: number } | null;
}

export interface ConnectDoorsReport {
  a: DoorReport;
  b: DoorReport;
  back: boolean;
}

/** One Pokémon of a new trainer's team: species and level (the game gives the moves of the level; no items). */
export interface PartyMon {
  species: number;
  level: number;
}

/** The input of „Neuer Trainer hier“ (`g4_kernel::recipes::new_trainer_here`): the tile, where it looks (`dir` 0 up, 1 down,
 * 2 left, 3 right) and how far it sees (1 to 15, default 3), class and name (at most 7 characters), the team (1 to 6), the battle
 * texts (`before` ends on `\r`; an empty text is not written) and the look (absent or null: the family's default trainer). */
export interface NewTrainerHereInput {
  header: number;
  x: number;
  z: number;
  dir: number;
  sight?: number;
  class: number;
  name: string;
  party: PartyMon[];
  texts: { before: string; lose: string; after: string };
  graphics_id?: number | null;
}

/** A warning about a text, as the kernel words it (`g4_kernel::text::Problem`; `line` and `col` 1-based). */
export interface TextProblem {
  severity: "error" | "warning" | "info";
  file: string;
  line: number;
  col: number;
  message: string;
}

/** What „Neuer Trainer hier“ did: trainer `trainer` (after the placeholder `placeholder` of the sight slot, if this call wrote one;
 * `script_extended` when the battle script got new slots), object `object` of `event_file` with battle script `script`, its
 * defeat flag `trainer_flag`, and the warnings about its battle texts. */
export interface NewTrainerHereReport {
  trainer: number;
  placeholder: number | null;
  script_extended: boolean;
  event_file: number;
  object: number;
  script: number;
  trainer_flag: number;
  text_problems: TextProblem[];
}
