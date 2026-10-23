import { Tabs as BaseTabs } from "@base-ui/react/tabs";
import type { KeyboardEvent, ReactNode } from "react";
import { IconButton } from "./IconButton";
import styles from "./Tabs.module.css";

export type TabItem = {
  id: string;
  label: string;
  count?: number;
  /** Shows a dot in place of the close button. */
  unsaved?: boolean;
  /** Accessible name of the close button, e.g. "Datei 0412 schließen". */
  closeLabel?: string;
};

export type TabsProps = {
  items: readonly TabItem[];
  value: string;
  onChange: (id: string) => void;
  /** Without it the tabs have no close button and ignore Delete and Cmd/Ctrl+W. */
  onClose?: (id: string) => void;
  /** Names the tab list for assistive technology. */
  ariaLabel: string;
  /** The content of the active tab. */
  children?: ReactNode;
};

/** Switching follows focus (arrow keys); Delete or Cmd/Ctrl+W on the focused tab closes it. */
export function Tabs({ items, value, onChange, onClose, ariaLabel, children }: TabsProps) {
  const closeOnKey = (id: string) => (event: KeyboardEvent) => {
    if (!onClose) return;
    const isCloseKey = event.key === "Delete" || ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "w");
    if (!isCloseKey) return;
    event.preventDefault();
    onClose(id);
  };
  return (
    <BaseTabs.Root className={styles.root} value={value} onValueChange={(next) => onChange(String(next))}>
      <BaseTabs.List className={styles.list} aria-label={ariaLabel} activateOnFocus>
        {items.map((item) => (
          <div key={item.id} role="presentation" className={styles.item} data-active={item.id === value ? "" : undefined}>
            <BaseTabs.Tab value={item.id} className={styles.tab} onKeyDown={closeOnKey(item.id)}>
              {item.label}
              {item.count !== undefined && <span className={styles.count}>{item.count}</span>}
            </BaseTabs.Tab>
            {item.unsaved ? (
              <span className={styles.slot}>
                <span className={`${styles.dot} ${onClose ? styles.swapDot : ""}`} role="img" aria-label="ungespeichert" />
                {onClose && (
                  <span className={styles.swapClose}>
                    <IconButton label={item.closeLabel ?? `${item.label} schließen`} icon="close" onClick={() => onClose(item.id)} />
                  </span>
                )}
              </span>
            ) : (
              onClose && <IconButton label={item.closeLabel ?? `${item.label} schließen`} icon="close" onClick={() => onClose(item.id)} />
            )}
          </div>
        ))}
      </BaseTabs.List>
      <BaseTabs.Panel value={value} className={styles.panel}>
        {children}
      </BaseTabs.Panel>
    </BaseTabs.Root>
  );
}
