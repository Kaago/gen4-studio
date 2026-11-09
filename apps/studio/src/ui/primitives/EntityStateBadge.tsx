import { useTranslation } from "react-i18next";
import { Badge } from "./Badge";
import type { BadgeKind } from "./Badge";

/** How an entity of the project stands against the base ROM, or as a tombstone (3d.4). */
export type EntityState = "changed" | "new" | "deleted";

/** The badge kind of each state; its text is `entityStateBadge.<state>` in the `ui` catalog. */
const KINDS: Record<EntityState, BadgeKind> = { changed: "warning", new: "info", deleted: "error" };

/** The state of an entity in a head: „geändert“ (warning), „neu“ (info), „gelöscht“ (error); `data-entity-state` names it. */
export function EntityStateBadge({ state }: { state: EntityState }) {
  const { t } = useTranslation("ui");
  return (
    <span data-entity-state={state}>
      <Badge kind={KINDS[state]}>{t(`entityStateBadge.${state}`)}</Badge>
    </span>
  );
}
