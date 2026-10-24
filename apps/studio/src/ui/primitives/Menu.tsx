import { Menu as BaseMenu } from "@base-ui/react/menu";
import type { ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./Menu.module.css";

export type MenuItemSpec = { label: string; icon?: IconName; onSelect: () => void; disabled?: boolean };
export type MenuEntry = MenuItemSpec | "separator";

export type MenuProps = {
  /** The content of the button that opens the menu (text, a symbol, a count). It is not interactive itself. */
  trigger: ReactNode;
  /** The accessible name of the trigger; required when `trigger` is a symbol without text. */
  label?: string;
  items: readonly MenuEntry[];
  /** Called with `true` when the menu opens and `false` when it closes (pick, Escape, click outside). */
  onOpenChange?: (open: boolean) => void;
};

/** Renders the entries of a menu; the context menu of `Reference` shares it. */
export function MenuEntries({ items }: { items: readonly MenuEntry[] }) {
  return items.map((entry, index) =>
    entry === "separator" ? (
      <BaseMenu.Separator key={`separator-${index}`} className={styles.separator} />
    ) : (
      <BaseMenu.Item key={`${index}-${entry.label}`} className={styles.item} disabled={entry.disabled} onClick={entry.onSelect}>
        {entry.icon && <Icon name={entry.icon} />}
        {entry.label}
      </BaseMenu.Item>
    ),
  );
}

export function Menu({ trigger, label, items, onOpenChange }: MenuProps) {
  return (
    <BaseMenu.Root onOpenChange={onOpenChange ? (open) => onOpenChange(open) : undefined}>
      <BaseMenu.Trigger className={styles.trigger} aria-label={label}>
        {trigger}
      </BaseMenu.Trigger>
      <BaseMenu.Portal>
        <BaseMenu.Positioner sideOffset={4} className={styles.positioner}>
          <BaseMenu.Popup className={styles.popup}>
            <MenuEntries items={items} />
          </BaseMenu.Popup>
        </BaseMenu.Positioner>
      </BaseMenu.Portal>
    </BaseMenu.Root>
  );
}
