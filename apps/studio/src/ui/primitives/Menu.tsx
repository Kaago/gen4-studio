import { ContextMenu } from "@base-ui/react/context-menu";
import { Menu as BaseMenu } from "@base-ui/react/menu";
import { useCallback, useRef } from "react";
import type { KeyboardEvent, ReactNode, RefObject } from "react";
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

export type ContextMenuPopupProps = {
  children: ReactNode;
  /** The accessible name of the menu (the row's name); without it the menu has none, it has no trigger to be labelled by. */
  label?: string;
  /** True while the menu opening now was opened by key (Shift+F10, the context menu key): it then starts on its first entry. */
  byKey?: RefObject<boolean>;
};

/**
 * The popup of a context menu, inside a `ContextMenu.Root` whose trigger is the right-clicked thing (`Reference`, `ListRow`, `TreeRow`).
 * The focus moves into the menu as it mounts: Base UI moves it only on the next animation frame, which a busy page delays and a hidden one
 * never runs, and keys pressed meanwhile reach the row (Enter picks it, the arrows walk the tree). Opened by key it starts on the first
 * entry (`firstEntry`), opened by the pointer on the menu itself, so the arrow keys walk the menu from there. Closed, the focus goes back
 * to where it was before (the row), unless an entry moved it on (to a panel or a dialog it opened).
 */
export function ContextMenuPopup({ children, label, byKey }: ContextMenuPopupProps) {
  const shown = useRef<HTMLDivElement | null>(null);
  const before = useRef<HTMLElement | null>(null);
  // Stable, so a render while the menu is open does not attach it again and pull the focus back.
  const focusIn = useCallback(
    (popup: HTMLDivElement | null) => {
      shown.current = popup;
      if (popup === null) return;
      const active = popup.ownerDocument.activeElement;
      // Attached again with the focus inside (Strict Mode does that), it keeps what it had.
      if (popup.contains(active)) return;
      before.current = active instanceof HTMLElement && active !== popup.ownerDocument.body ? active : null;
      popup.focus({ preventScroll: true });
      if (!byKey?.current) return;
      byKey.current = false;
      // The entries take part in the menu's list once this render is through: focused then, the entry is highlighted as well.
      queueMicrotask(() => {
        if (popup.contains(popup.ownerDocument.activeElement)) firstEntry(popup)?.focus({ preventScroll: true });
      });
    },
    [byKey],
  );
  // Base UI would go back to the element focused when its focus handling started: the menu itself, by then.
  const focusBack = useCallback(() => {
    const active = shown.current?.ownerDocument.activeElement;
    if (active && active !== active.ownerDocument.body && !shown.current?.contains(active)) return false;
    return before.current?.isConnected ? before.current : true;
  }, []);
  return (
    <ContextMenu.Portal>
      <ContextMenu.Positioner className={styles.positioner}>
        <BaseMenu.Popup ref={focusIn} className={styles.popup} aria-label={label} finalFocus={focusBack}>
          {children}
        </BaseMenu.Popup>
      </ContextMenu.Positioner>
    </ContextMenu.Portal>
  );
}

/**
 * The entry of the menu `popup` a context menu opened by key starts on: the first one that can be picked, else the first one (WAI-ARIA lets
 * a locked entry hold the focus, so a screen reader reads it and its reason, „Kein Original“). Null for a menu without entries.
 */
function firstEntry(popup: HTMLElement | null): HTMLElement | null {
  const entries = [...(popup?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
  return entries.find((entry) => entry.getAttribute("aria-disabled") !== "true" && !entry.hasAttribute("data-disabled")) ?? entries[0] ?? null;
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
