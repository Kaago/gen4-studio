import { Badge } from "./Badge";
import type { BadgeKind } from "./Badge";

/** How an entity of the project stands against the base ROM, or as a tombstone (3d.4). */
export type EntityState = "changed" | "new" | "deleted";

const LOOKS: Record<EntityState, { kind: BadgeKind; text: string }> = {
  changed: { kind: "warning", text: "geändert" },
  new: { kind: "info", text: "neu" },
  deleted: { kind: "error", text: "gelöscht" },
};

/**
 * The state of an entity in a head: „geändert“ (warning), „neu“ (info), „gelöscht“ (error); `data-entity-state` names it. `compact`: the
 * badge with its symbol only, the word as its name and tooltip, for a one-line list row whose name and number need the room.
 */
export function EntityStateBadge({ state, compact = false }: { state: EntityState; compact?: boolean }) {
  const { kind, text } = LOOKS[state];
  if (compact) {
    return (
      <span data-entity-state={state} role="img" aria-label={text} title={text}>
        <Badge kind={kind}>{""}</Badge>
      </span>
    );
  }
  return (
    <span data-entity-state={state}>
      <Badge kind={kind}>{text}</Badge>
    </span>
  );
}
