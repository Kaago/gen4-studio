import { tileBehaviors } from "../../api";
import type { TileBehavior } from "../../api";

let namesCache: Promise<Map<number, TileBehavior>> | null = null;
/**
 * The behaviour table of the open project's game family is static for that project, so it is fetched once; a failed
 * fetch is retried on the next load. `resetTileNames` drops it when another project is opened (the family may differ).
 */
export function loadNames(): Promise<Map<number, TileBehavior>> {
  if (namesCache) return namesCache;
  const pending: Promise<Map<number, TileBehavior>> = tileBehaviors()
    .then((list) => new Map(list.map((b) => [b.value, b] as const)))
    .catch((e) => {
      // Only forget this fetch: after a reset the cache may already hold the next project's.
      if (namesCache === pending) namesCache = null;
      throw e;
    });
  namesCache = pending;
  return pending;
}

/** Forgets the cached table (another project was opened). A fetch still in flight no longer fills the cache. */
export function resetTileNames() {
  namesCache = null;
}
