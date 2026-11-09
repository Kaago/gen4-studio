import { useState } from "react";
import type { AssetId, HeaderSummary, ProjectInfo, StoryReport } from "../api";
import { t } from "../i18n";
import { NewStoryDialog, createStory } from "../stories/NewStoryDialog";
import type { NewStoryRequest } from "../stories/NewStoryDialog";
import { StoryRecipeDialog } from "../stories/recipes/StoryRecipeDialog";
import type { StoryRecipeRequest } from "../stories/recipes/storyRecipeModel";
import { useStories } from "../stories/useStories";
import { toast } from "../ui/frame/toasts";

export type StoryRecipeHostProps = {
  /** The step recipe asked for (null: none, the host shows nothing). */
  request: StoryRecipeRequest | null;
  onClose: () => void;
  project: ProjectInfo | null;
  /** Changes when the project may differ: the stories, the story and the figures are read again. */
  revision: number;
  headers: readonly HeaderSummary[];
  /** The recipe went in (one undo step that also named a flag or a variable): the app reads again what it changed. */
  onApplied: (changed: AssetId[]) => Promise<void> | void;
  onError: (e: unknown) => void;
  /** „Neue Geschichte…“ made a story (one undo step): the app's toast and what it reads again. */
  onStoryCreated?: (report: StoryReport, name: string) => void;
  /** Runs before a recipe (the app saves typing in the place's script file, which the recipe appends to). */
  beforeApply?: (header: number) => Promise<void>;
};

/**
 * The step recipes of 3d.7 as dialogs over the app (the area Geschichten has no deck to put a wizard into): the open request's dialog
 * with the stories of the project, „Neue Geschichte…“ over it (the dialog keeps its fields meanwhile and takes the new story), and after
 * a recipe went in the toast „Schritt „{name}“ angelegt“. Each request opens its dialog anew.
 */
export function StoryRecipeHost({ request, onClose, project, revision, headers, onApplied, onError, onStoryCreated, beforeApply }: StoryRecipeHostProps) {
  const [creating, setCreating] = useState(false);
  const [made, setMade] = useState<string | null>(null);
  const { list, error, reload } = useStories({ project: request === null ? null : project, revision });
  // Another request starts without what the last one made.
  const [seen, setSeen] = useState(request);
  if (seen !== request) {
    setSeen(request);
    setCreating(false);
    setMade(null);
  }
  if (request === null) return null;
  const create = async (story: NewStoryRequest) => {
    const { report, name } = await createStory(story);
    onStoryCreated?.(report, name);
    for (const warning of report.warnings) onError(warning);
    reload();
    // Only a story with a variable takes step recipes.
    if (story.via === "recipe") setMade(report.id);
  };
  return (
    <>
      <StoryRecipeDialog
        key={JSON.stringify(request)}
        request={request}
        stories={list?.stories ?? null}
        storiesError={error}
        project={project}
        revision={revision}
        headers={headers}
        onApplied={(outcome, applied) => {
          toast.info(t("app:storyRecipe.stepCreated", { name: applied.name }));
          onClose();
          void Promise.resolve(onApplied(outcome.changed)).catch(onError);
        }}
        onClose={onClose}
        onError={onError}
        beforeApply={beforeApply}
        onNewStory={() => setCreating(true)}
        newStory={made}
        hidden={creating}
      />
      <NewStoryDialog open={creating} onClose={() => setCreating(false)} onCreate={create} project={project} revision={revision} />
    </>
  );
}
