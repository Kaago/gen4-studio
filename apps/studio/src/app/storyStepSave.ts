import type { HeaderSummary } from "../api";

/**
 * The script files a step recipe of 3d.7 rewrites for place `header`: its script file (talk, reward, the visibility block and the scene
 * are appended there) and its level script file when it has one (`figure_appears` and `scene_on_enter` add an entry or a condition).
 * Empty for a header the list does not know.
 */
export function storyStepFiles(headers: readonly HeaderSummary[], header: number): number[] {
  const place = headers.find((h) => h.id === header);
  if (place === undefined) return [];
  return place.level_script_id === 0 ? [place.script_file_id] : [place.script_file_id, place.level_script_id];
}

/**
 * Saves typing in the files a step recipe rewrites ([`storyStepFiles`]) before it runs, one after the other: the recipe works from the
 * files on disk, and typed text saved after it would drop what it added. A failed save rejects (the recipe does not run).
 */
export async function saveBeforeStoryStep(headers: readonly HeaderSummary[], header: number, saveIfDirty: (file: number) => Promise<void>): Promise<void> {
  for (const file of storyStepFiles(headers, header)) await saveIfDirty(file);
}
