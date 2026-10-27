import { ContextMenu } from "@base-ui/react/context-menu";
import { Menu as BaseMenu } from "@base-ui/react/menu";
import type { KeyboardEvent, ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./Menu.module.css";

export type MenuItemSpec = {
  label: string;
  icon?: IconName;
  onSelect: () => void;
  disabled?: boolean;
  /** Makes the item one of a choice (`menuitemradio`): neighbouring items with `checked` form one group, a check marks the chosen one. */
  checked?: boolean;
  /** Quiet text at the end of the item: a count, a shortcut. */
  detail?: ReactNode;
};
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

function ItemContent({ entry }: { entry: MenuItemSpec }) {
  return (
    <>
      {entry.icon && <Icon name={entry.icon} />}
      <span className={styles.label}>{entry.label}</span>
      {entry.detail !== undefined && <span className={styles.detail}>{entry.detail}</span>}
    </>
  );
}

/** Splits the entries into runs: a separator, a plain item, or neighbouring items with `checked` (one radio group). */
function runsOf(items: readonly MenuEntry[]): (MenuEntry | MenuItemSpec[])[] {
  const runs: (MenuEntry | MenuItemSpec[])[] = [];
  for (const entry of items) {
    const last = runs[runs.length - 1];
    if (entry !== "separator" && entry.checked !== undefined) {
      if (Array.isArray(last)) last.push(entry);
      else runs.push([entry]);
    } else runs.push(entry);
  }
  return runs;
}

/** Renders the entries of a menu; the context menu of `Reference` shares it. */
export function MenuEntries({ items }: { items: readonly MenuEntry[] }) {
  return runsOf(items).map((run, index) => {
    if (run === "separator") return <BaseMenu.Separator key={`separator-${index}`} className={styles.separator} />;
    if (Array.isArray(run)) {
      const chosen = run.findIndex((entry) => entry.checked);
      return (
        <BaseMenu.RadioGroup key={`radio-${index}`} value={chosen < 0 ? null : chosen}>
          {run.map((entry, at) => (
            <BaseMenu.RadioItem
              key={`${at}-${entry.label}`}
              value={at}
              closeOnClick
              className={styles.item}
              disabled={entry.disabled}
              onClick={entry.onSelect}
            >
              <span className={styles.check}>{entry.checked && <Icon name="check" />}</span>
              <ItemContent entry={entry} />
            </BaseMenu.RadioItem>
          ))}
        </BaseMenu.RadioGroup>
      );
    }
    return (
      <BaseMenu.Item key={`${index}-${run.label}`} className={styles.item} disabled={run.disabled} onClick={run.onSelect}>
        <ItemContent entry={run} />
      </BaseMenu.Item>
    );
  });
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

/** The popup of a context menu, inside a `ContextMenu.Root` whose trigger is the right-clicked thing (`Reference`, `ListRow`, `TreeRow`). */
export function ContextMenuPopup({ children }: { children: ReactNode }) {
  return (
    <ContextMenu.Portal>
      <ContextMenu.Positioner className={styles.positioner}>
        <BaseMenu.Popup className={styles.popup}>{children}</BaseMenu.Popup>
      </ContextMenu.Positioner>
    </ContextMenu.Portal>
  );
}

/**
 * Shift+F10 and the context menu key on the focused element open its context menu at its lower edge, as a right click there would (the
 * way the deck does it for its tabs). Only for the element itself, not for a control inside it; true when it opened the menu.
 */
export function openContextMenuByKey(event: KeyboardEvent<HTMLElement>): boolean {
  const byKey = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);
  if (!byKey || event.target !== event.currentTarget) return false;
  event.preventDefault();
  const box = event.currentTarget.getBoundingClientRect();
  event.currentTarget.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: box.left, clientY: box.bottom }));
  return true;
}
