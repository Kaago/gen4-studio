import { Icon, Menu } from "../ui/primitives";
import type { MenuEntry } from "../ui/primitives";
import styles from "./HeadActions.module.css";

/**
 * The menu „Aktionen“ of a head that is no `InspectorHead` (the views of trainers, Pokédex and Begegnungen, the texts' hand, the script
 * panel), in the inspector head's pattern: a symbol button at the right end of a flex row. Left out when there is nothing in it.
 */
export function HeadActions({ items, push = true }: { items: readonly MenuEntry[]; /** Pushed to the right end of the row (default). */ push?: boolean }) {
  if (items.length === 0) return null;
  return (
    <span className={push ? `${styles.actions} ${styles.push}` : styles.actions}>
      <Menu label="Aktionen" trigger={<Icon name="more" />} items={items} />
    </span>
  );
}
