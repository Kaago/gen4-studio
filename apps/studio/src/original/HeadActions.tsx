import { EntityStateBadge, Icon, Menu } from "../ui/primitives";
import type { EntityState, MenuEntry } from "../ui/primitives";
import styles from "./HeadActions.module.css";

/**
 * The menu „Aktionen“ of a head that is no `InspectorHead` (the views of trainers, Pokédex and Begegnungen, the texts' hand, the script
 * panel), in the inspector head's pattern: a symbol button at the right end of a flex row, with the entity's `state` badge before it. Left
 * out when there is nothing in it.
 */
export function HeadActions({
  items,
  push = true,
  state = null,
}: {
  items: readonly MenuEntry[];
  /** Pushed to the right end of the row (default). */
  push?: boolean;
  /** How the entity stands against the base ROM or as a tombstone: „geändert“, „neu“, „gelöscht“ beside the menu. */
  state?: EntityState | null;
}) {
  if (items.length === 0 && state === null) return null;
  return (
    <span className={push ? `${styles.actions} ${styles.push}` : styles.actions}>
      {state !== null && <EntityStateBadge state={state} />}
      {items.length > 0 && <Menu label="Aktionen" trigger={<Icon name="more" />} items={items} />}
    </span>
  );
}
