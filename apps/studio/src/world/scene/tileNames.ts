import { tileBehaviors } from "../../api";
import type { TileBehavior } from "../../api";

let namesCache: Promise<Map<number, TileBehavior>> | null = null;
/** The behaviour table is static, so it is fetched once; a failed fetch is retried on the next load. */
export function loadNames(): Promise<Map<number, TileBehavior>> {
  namesCache ??= tileBehaviors()
    .then((list) => new Map(list.map((b) => [b.value, b] as const)))
    .catch((e) => {
      namesCache = null;
      throw e;
    });
  return namesCache;
}
