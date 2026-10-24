import type { ReactNode } from "react";
import { Icon } from "./Icon";
import { Kbd } from "./Kbd";
import { Menu } from "./Menu";
import type { MenuItemSpec } from "./Menu";
import styles from "./PanelHead.module.css";

export type PanelHeadSwitcherItem = {
  id: string;
  label: string;
  /** How many entries the part has, shown in the menu. */
  count?: number;
  /** Its key combination, e.g. "⌘1". */
  shortcut?: string;
  disabled?: boolean;
  /** Why a locked part is locked; the menu says it after the label ("Trainer, kommt mit Phase 3b.3b"). */
  hint?: string;
};

export type PanelHeadSwitcher = {
  items: readonly PanelHeadSwitcherItem[];
  value: string;
  onChange: (id: string) => void;
};

export type PanelHeadProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  /** A state shown at the right end (a badge); in a narrow head it stays beside the title and the actions take their own row. */
  status?: ReactNode;
  /** The parts of an area (Pokédex, Trainer, …): the heading becomes a menu button that names the current part and switches. */
  switcher?: PanelHeadSwitcher;
};

function switcherEntry(item: PanelHeadSwitcherItem, switcher: PanelHeadSwitcher): MenuItemSpec {
  const detail =
    item.count !== undefined || item.shortcut !== undefined ? (
      <>
        {item.count !== undefined && <span>{item.count}</span>}
        {item.shortcut !== undefined && <Kbd keys={item.shortcut} />}
      </>
    ) : undefined;
  return {
    label: item.disabled && item.hint ? `${item.label}, ${item.hint}` : item.label,
    checked: item.id === switcher.value,
    disabled: item.disabled,
    detail,
    onSelect: () => {
      if (!item.disabled) switcher.onChange(item.id);
    },
  };
}

/** The own head of a panel whose dock group hides the tab strip. */
export function PanelHead({ title, subtitle, actions, status, switcher }: PanelHeadProps) {
  const current = switcher?.items.find((item) => item.id === switcher.value);
  return (
    <header className={status ? `${styles.head} ${styles.withStatus}` : styles.head}>
      <h2 className={styles.title}>
        {switcher ? (
          <Menu
            trigger={
              <>
                {current?.label ?? title}
                <Icon name="caretDown" />
              </>
            }
            items={switcher.items.map((item) => switcherEntry(item, switcher))}
          />
        ) : (
          title
        )}
      </h2>
      {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
      {actions && <div className={styles.actions}>{actions}</div>}
      {status && (
        <div className={styles.status} data-panel-status="">
          {status}
        </div>
      )}
    </header>
  );
}
