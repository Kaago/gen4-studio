import type { TextureIssue } from "../api";
import { t } from "../i18n";

/** How many map models are fetched and parsed at the same time (each one may run apicula). */
export const MODEL_CONCURRENCY = 4;

/**
 * Runs `tasks` with at most `cap` of them in flight, starting them in list order. Never rejects:
 * every task's outcome is in the result, in task order. `onProgress` is called after each task
 * settled, with the number settled so far and the total.
 */
export async function limitConcurrency<T>(
  tasks: (() => Promise<T>)[],
  cap: number,
  onProgress?: (done: number, total: number) => void,
): Promise<PromiseSettledResult<T>[]> {
  const results: PromiseSettledResult<T>[] = new Array(tasks.length);
  let next = 0;
  let done = 0;
  const worker = async () => {
    while (next < tasks.length) {
      const i = next++;
      try {
        results[i] = { status: "fulfilled", value: await tasks[i]() };
      } catch (reason) {
        results[i] = { status: "rejected", reason };
      }
      onProgress?.(++done, tasks.length);
    }
  };
  await Promise.all(Array.from({ length: Math.min(Math.max(cap, 1), tasks.length) }, worker));
  return results;
}

/**
 * One status note for all texture problems met while loading a matrix, one entry per distinct
 * tileset (every map of an area shares the same one); null when there were none.
 */
export function textureNote(issues: readonly TextureIssue[]): string | null {
  const parts = new Map<string, string>();
  for (const { tileset } of issues) {
    const key = tileset === null ? "area" : String(tileset);
    if (parts.has(key)) continue;
    parts.set(
      key,
      tileset === null ? t("world:load.areaDataUnreadable") : t("world:load.tilesetNotConvertible", { tileset: String(tileset).padStart(4, "0") }),
    );
  }
  return parts.size === 0 ? null : t("world:load.mapsUntextured", { problems: [...parts.values()].join(", ") });
}
