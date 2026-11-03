import type { FlagKind, ProjectInfo, StoryRowDto } from "../api";
import { useFlagList } from "../flags/useFlagList";
import { flagIdOf } from "./storyWords";
import { useStories } from "./useStories";
import { useStory } from "./useStory";

/** The story that holds a flag (among its further flags, or as its progress) or a variable (as its progress), by the number's id. */
function storyOf(rows: readonly StoryRowDto[], kind: FlagKind, id: string): StoryRowDto | undefined {
  return rows.find((row) => {
    if (row.problem !== null) return false;
    if (kind === "flag" && row.flags.includes(id)) return true;
    const progress = row.progress;
    return progress !== null && (kind === "flag" ? "flag" in progress && progress.flag === id : "var" in progress && progress.var === id);
  });
}

/**
 * The line under a figure's visibility flag or a trigger's variable in Erzählen: „Geschichte „X“: {Schrittname}“ when a story holds the
 * number (resolved by the id of its entry: its own name, else its pret name), with the step that uses the flag, or for a variable the step
 * of `value` (the value the trigger fires at); „Geschichte „X““ when no step says more. Null when no story holds it (0 is none),
 * undefined while that is not known yet (no project, the lists still read). The flag list is the one the field reads (`source` events).
 */
export function useStoryHint(kind: FlagKind, number: number, deps: { project: ProjectInfo | null; revision: number; value?: number }): string | null | undefined {
  const { project, revision, value } = deps;
  const { list } = useFlagList(kind, { project: number === 0 ? null : project, revision, source: "events" });
  const id = number === 0 || list === null ? null : flagIdOf(kind, number, list.entries.find((e) => e.number === number));
  const stories = useStories({ project: id === null ? null : project, revision });
  const row = id === null || stories.list === null ? undefined : storyOf(stories.list.stories, kind, id);
  const { story } = useStory(row?.id ?? null, { project, revision });
  if (number === 0) return null;
  if (project === null || id === null) return undefined;
  if (stories.list === null) return stories.error === null ? undefined : null;
  if (row === undefined) return null;
  const steps = story?.story.id === row.id ? (story.story.steps ?? []) : [];
  const step = kind === "flag" ? steps.find((s) => s.uses?.some((u) => "flag" in u && u.flag === id)) : value === undefined ? undefined : steps.find((s) => s.value === value);
  return step === undefined ? `Geschichte „${row.name}“` : `Geschichte „${row.name}“: ${step.name}`;
}
