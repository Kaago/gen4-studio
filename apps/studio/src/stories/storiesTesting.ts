import type { StoryListDto, StoryRowDto, ThreadDto, ThreadGraphDto, ThreadListDto } from "../api";
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

/** One story row as `story_list` sends it: readable, with a variable as progress and no steps, unless `over` says otherwise. */
export function storyRow(number: number, over: Partial<StoryRowDto> = {}): StoryRowDto {
  return { number, id: `STORY_${String(number).padStart(4, "0")}`, name: `Geschichte ${number}`, progress: { var: "VAR_RIVALIN" }, steps: 0, problem: null, ...over };
}

/** A list of `stories` of a text project. */
export function storiesOf(stories: StoryRowDto[], over: Partial<StoryListDto> = {}): StoryListDto {
  return { stories, text_project: true, ms: 3, ...over };
}
