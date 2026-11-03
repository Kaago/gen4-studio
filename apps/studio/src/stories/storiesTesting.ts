import type { Story, StoryDto, StoryListDto, StoryReport, StoryRowDto, ThreadDto, ThreadGraphDto, ThreadListDto } from "../api";
import { sampleThreadGraph } from "./graph/sampleGraph";

/** One thread as `list_threads` sends it: an unnamed variable with three values at three places (two headers), no story, unless `over` says otherwise. */
export function threadOf(variable: number, over: Partial<ThreadDto> = {}): ThreadDto {
  return {
    var: variable,
    name: null,
    own_id: null,
    values: [0, 1, 2],
    places: 3,
    headers: [411, 412],
    header_names: ["Zweiblattdorf", "Route 201"],
    story: null,
    ...over,
  };
}

/** A list of `threads` of a text project with adopted scripts, unless `over` says otherwise. */
export function threadList(threads: ThreadDto[], over: Partial<ThreadListDto> = {}): ThreadListDto {
  return { threads, text_project: true, scripts_adopted: true, ms: 7, ...over };
}

/** Three threads: one with a pret name, one with an own name and a story, one with sixteen values at seven places. */
export const THREADS: ThreadListDto = threadList([
  threadOf(0x4031, { name: "VAR_UNK_0x4031" }),
  threadOf(0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN", story: "STORY_RIVALIN" }),
  threadOf(0x4055, {
    values: Array.from({ length: 16 }, (_, i) => i),
    places: 7,
    headers: [1, 2, 3, 4, 5],
    header_names: ["Herzhofen", "Route 205", "Flori", "Ewigenau", "Route 210"],
  }),
]);

/** The graph of `variable` (the sample's nodes and edges under another number), unless `over` says otherwise. */
export function graphOf(variable: number, over: Partial<ThreadGraphDto> = {}): ThreadGraphDto {
  return { ...sampleThreadGraph, var: variable, ...over };
}

/** One story row as `story_list` sends it: readable, with a variable as progress, no steps and no flags, unless `over` says otherwise. */
export function storyRow(number: number, over: Partial<StoryRowDto> = {}): StoryRowDto {
  return { number, id: `STORY_${String(number).padStart(4, "0")}`, name: `Geschichte ${number}`, progress: { var: "VAR_RIVALIN" }, steps: 0, problem: null, flags: [], ...over };
}

/** A list of `stories` of a text project, without the pool (no index), unless `over` says otherwise. */
export function storiesOf(stories: StoryRowDto[], over: Partial<StoryListDto> = {}): StoryListDto {
  return { stories, text_project: true, free_vars: null, ms: 3, ...over };
}

/** A story as `story_get` sends it: „Rivalin“ on VAR_RIVALIN (0x4040) with three steps and notes, unless `over` says otherwise. */
export function storyDto(number: number, over: Partial<StoryDto> = {}, story: Partial<Story> = {}): StoryDto {
  return {
    number,
    story: {
      id: `STORY_${String(number).padStart(4, "0")}`,
      name: "Rivalin",
      notes: "Die Rivalin begleitet den Weg.",
      progress: { var: "VAR_RIVALIN" },
      steps: [
        { key: "s1", value: 1, kind: "dialog", name: "Rivalin trifft", uses: [{ map: "MAP_ZWEIBLATTDORF" }, { message: { archive: 12, index: 4 } }] },
        { key: "s2", value: 2, kind: "battle", name: "Kampf am See", uses: [{ trainer: "TRAINER_0042" }, { script: { file: 101, slot: 2 } }] },
        {
          key: "s3",
          value: 3,
          kind: "reward",
          name: "Pokédex bekommen",
          uses: [{ flag: "FLAG_HAT_MEDAILLE" }, { var: "VAR_16448" }, { object: { file: 394, local_id: 3 } }, { levelScript: { file: 12, condition: 1 } }],
        },
      ],
      ...story,
    },
    layout: { nodes: {} },
    progress_number: 0x4040,
    ms: 2,
    ...over,
  };
}

/** What `story_create` and `story_update` answer: the story's three files of one undo step, unless `over` says otherwise. */
export function storyReport(number: number, over: Partial<StoryReport> = {}): StoryReport {
  const file = String(number).padStart(4, "0");
  return {
    id: `STORY_${file}`,
    number,
    files: ["data/ids/stories.json", `data/stories/${file}.json`, `data/stories/${file}.layout.json`],
    change: 3,
    ms: 2600,
    warnings: [],
    ...over,
  };
}
