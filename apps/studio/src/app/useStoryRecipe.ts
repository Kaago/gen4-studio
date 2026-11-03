import { useCallback, useEffect, useState } from "react";
import type { ProjectInfo } from "../api";
import type { StoryRecipeRequest } from "../stories/recipes/storyRecipeModel";

/**
 * The step recipe the app shows as a dialog (null: none): a panel asks for one (`open`, a stable function: the step bar of a story,
 * „Zu einer Geschichte“ after „Neue Figur“, „Dialog anlegen…“ of a figure), the dialog closes it. Another project (or none) closes it.
 */
export function useStoryRecipe(project: ProjectInfo | null): { request: StoryRecipeRequest | null; open: (request: StoryRecipeRequest) => void; close: () => void } {
  const [request, setRequest] = useState<StoryRecipeRequest | null>(null);
  useEffect(() => setRequest(null), [project]);
  const close = useCallback(() => setRequest(null), []);
  return { request, open: setRequest, close };
}
