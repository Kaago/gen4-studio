import { Badge } from "./Badge";
import type { BadgeKind } from "./Badge";

/** How an entity of the project stands against the base ROM, or as a tombstone (3d.4). */
export type EntityState = "changed" | "new" | "deleted";

const LOOKS: Record<EntityState, { kind: BadgeKind; text: string }> = {
  changed: { kind: "warning", text: "geändert" },
  new: { kind: "info", text: "neu" },
  deleted: { kind: "error", text: "gelöscht" },
};

/** The state of an entity in a head: „geändert“ (warning), „neu“ (info), „gelöscht“ (error); `data-entity-state` names it. */
export function EntityStateBadge({ state }: { state: EntityState }) {
  const { kind, text } = LOOKS[state];
  return (
    <span data-entity-state={state}>
      <Badge kind={kind}>{text}</Badge>
    </span>
  );
}
