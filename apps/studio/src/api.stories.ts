import type { AssetId, FlagAccess, NavTargetDto } from "./api";

/**
 * Stories and threads (3d.6), mirroring `apps/studio/src-tauri/src/commands/stories.rs` and `g4_kernel::stories`. Field names are the
 * Rust ones (snake_case, `levelScript` as the story file has it). Types only for now; the commands (`list_threads`, `thread_graph`,
 * `story_list`, `story_get`, `story_create`, `story_update`) and the re-export from `api.ts` follow with the area Geschichten.
 */

/** What a step of a story is: the seven kinds (`appearance` is appearing or disappearing; the step's name says which). */
export type StepKind = "start" | "dialog" | "condition" | "battle" | "reward" | "progress" | "appearance";

/** The number a story's progress is kept in, by its id. */
export type Progress = { var: string } | { flag: string };

/** One thing of the game a step uses, exactly one key each. */
export type StoryUse =
  | { trainer: string }
  | { map: string }
  | { flag: string }
  | { var: string }
  | { object: { file: number; local_id: number } }
  | { script: { file: number; slot: number } }
  | { message: { archive: number; index: number } }
  | { levelScript: { file: number; condition: number } };

/** One step of a story: a value of its progress. Empty `notes` and `uses` and a missing `made` are left out of the file. */
export interface Step {
  key: string;
  value: number;
  kind: StepKind;
  name: string;
  notes?: string;
  made?: { recipe: string; studio: string };
  uses?: StoryUse[];
}

/** A story: `data/stories/NNNN.json`. */
export interface Story {
  id: string;
  name: string;
  notes?: string;
  progress: Progress;
  steps?: Step[];
  flags?: string[];
}

/** The side file `data/stories/NNNN.layout.json`: the position of each node by the key of its step. */
export interface Layout {
  nodes: Record<string, { x: number; y: number }>;
}

/** One thread of the overview: the variable, its names, its values, at how many places, the headers among them and the story on it. */
export interface ThreadDto {
  var: number;
  name: string | null;
  own_id: string | null;
  values: number[];
  places: number;
  headers: number[];
  header_names: string[];
  story: string | null;
}

/** `list_threads`. Without adopted scripts the index sees no script and the list is empty. */
export interface ThreadListDto {
  threads: ThreadDto[];
  text_project: boolean;
  scripts_adopted: boolean;
  ms: number;
}

/** A value node (`v:<value>`), with the story's step on it; a value only a step names is `noted_only`. */
export interface ValueNodeDto {
  kind: "value";
  id: string;
  value: number;
  step: Step | null;
  noted_only: boolean;
}

/** A place node (`p:<kind>:<index>`): the source, its header, the label, whether it has no edge, and where a click leads. */
export interface PlaceNodeDto {
  kind: "place";
  id: string;
  source: AssetId;
  header: number | null;
  header_name: string | null;
  label: string;
  loose: boolean;
  nav: NavTargetDto;
}

/** A node of a thread's graph. */
export type GraphNodeDto = ValueNodeDto | PlaceNodeDto;

/** An edge: place → value where the place sets it („gesetzt in“), value → place where it checks it („geprüft in“); `field` and `slot` say where. */
export interface GraphEdgeDto {
  from: string;
  to: string;
  value: number;
  access: FlagAccess;
  field: string;
  slot: number | null;
}

/** `thread_graph`: the graph of one variable, with the story's layout when it was asked with a story. */
export interface ThreadGraphDto {
  var: number;
  name: string | null;
  own_id: string | null;
  nodes: GraphNodeDto[];
  edges: GraphEdgeDto[];
  layout: Layout | null;
  ms: number;
}

/** One story of the list; a file that does not read has its `problem` instead (empty name, no progress, no steps). */
export interface StoryRowDto {
  number: number;
  id: string;
  name: string;
  progress: Progress | null;
  steps: number;
  problem: string | null;
}

/** `story_list`. */
export interface StoryListDto {
  stories: StoryRowDto[];
  text_project: boolean;
  ms: number;
}

/** `story_get`: the story, its side file and the number of its progress (null when the id names nothing). */
export interface StoryDto {
  number: number;
  story: Story;
  layout: Layout;
  progress_number: number | null;
  ms: number;
}

/** What `story_create` and `story_update` did: id, number, files of the step, the change, and what failed after writing. */
export interface StoryReport {
  id: string;
  number: number;
  files: string[];
  change: number | null;
  ms: number;
  warnings: string[];
}
