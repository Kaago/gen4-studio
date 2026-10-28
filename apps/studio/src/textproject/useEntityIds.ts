import { useEffect, useState } from "react";
import type { EntityKind } from "../api";
import type { EntityIdCache } from "./TextProjectContext";
import { useTextProject } from "./TextProjectContext";

/**
 * The ids of `kind` by number, read once per kind for all heads (the text project's cache); null while they are read, when they cannot be
 * and when not `enabled` (a DSPRE folder has no register ids, so nothing is read there).
 */
export function useEntityIds(kind: EntityKind, enabled: boolean): Record<number, string> | null {
  const { ids } = useTextProject();
  const [read, setRead] = useState<{ cache: EntityIdCache; kind: EntityKind; ids: Record<number, string> } | null>(null);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    ids.read(kind).then(
      (found) => live && setRead({ cache: ids, kind, ids: found }),
      () => {
        /* no id line then; the head stays as in a DSPRE folder */
      },
    );
    return () => {
      live = false;
    };
  }, [ids, kind, enabled]);
  return enabled && read !== null && read.cache === ids && read.kind === kind ? read.ids : null;
}
