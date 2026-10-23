import { Combobox } from "@base-ui/react/combobox";
import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import { Dialog } from "./Dialog";
import { Icon } from "./Icon";
import { areaIcon } from "./Reference";
import { Segmented } from "./Segmented";
import { filterItems } from "./pickerFilter";
import type { PickerItem } from "./pickerFilter";
import styles from "./Picker.module.css";

export type { PickerItem } from "./pickerFilter";

export type PickerProps = {
  open: boolean;
  /** Called with `false` after a pick, for Escape, the close button and a click outside; the owner decides. */
  onOpenChange: (open: boolean) => void;
  title: string;
  items: readonly PickerItem[];
  /** Called with the chosen item; the picker then asks the owner to close. */
  onPick: (item: PickerItem) => void;
  /** Narrows `items` by kind. The owner filters `items`; the picker only shows the choice. */
  filters?: readonly { value: string; label: string }[];
  filter?: string;
  onFilter?: (value: string) => void;
  placeholder?: string;
  /** Below the list, e.g. a hint or a button for "Neu anlegen". */
  footer?: ReactNode;
  /** Shown instead of the list when nothing matches. */
  emptyText?: string;
};

type Group = { value: string; items: PickerItem[] };

/** Items by `group` in order of first appearance; items without a group come under an unnamed first group. */
function groupItems(items: readonly PickerItem[]): Group[] {
  const groups = new Map<string, PickerItem[]>();
  for (const item of items) {
    const key = item.group ?? "";
    const list = groups.get(key);
    if (list) list.push(item);
    else groups.set(key, [item]);
  }
  return [...groups].map(([value, list]) => ({ value, items: list }));
}

function PickerRow({ item }: { item: PickerItem }) {
  return (
    <Combobox.Item value={item} className={styles.row}>
      {item.preview && <span className={styles.preview}>{item.preview}</span>}
      <span className={`${styles.area} ${styles[item.area]}`}>
        <Icon name={areaIcon[item.area]} />
      </span>
      <span className={styles.label}>{item.label}</span>
      {item.tech && <span className={styles.tech}>{item.tech}</span>}
      {item.count && <span className={styles.count}>{item.count}</span>}
    </Combobox.Item>
  );
}

/** Mounted only while the dialog is open, so the query and the highlight start fresh every time. */
function PickerBody({ onOpenChange, items, onPick, filters, filter, onFilter, placeholder, footer, emptyText }: Omit<PickerProps, "open" | "title">) {
  const [query, setQuery] = useState("");
  const groups = useMemo(() => groupItems(filterItems(items, query)), [items, query]);
  const grouped = groups.some((g) => g.value !== "");
  return (
    <div className={styles.picker}>
      <Combobox.Root<PickerItem>
        inline
        open
        autoHighlight
        items={grouped ? groups : groups.flatMap((g) => g.items)}
        filter={null}
        value={null}
        itemToStringLabel={(item) => item.label}
        onInputValueChange={(text, details) => {
          // Picking fills the input with the label; the query stays what the user typed.
          if (details.reason === "input-change" || details.reason === "input-clear") setQuery(text);
        }}
        onValueChange={(item) => {
          if (!item) return;
          onPick(item);
          onOpenChange(false);
        }}
        onOpenChange={(next) => {
          if (!next) onOpenChange(false);
        }}
      >
        <div className={styles.search}>
          <Icon name="search" />
          <Combobox.Input className={styles.input} aria-label="Suchen" placeholder={placeholder ?? "Suchen …"} spellCheck={false} />
        </div>
        {filters && filters.length > 0 && filter !== undefined && onFilter && (
          <div className={styles.filters}>
            <Segmented size="s" ariaLabel="Filter" options={filters} value={filter} onChange={onFilter} />
          </div>
        )}
        <Combobox.List className={styles.list} aria-label="Treffer">
          {grouped
            ? (group: Group) => (
                <Combobox.Group key={group.value} items={group.items} className={styles.group}>
                  {group.value !== "" && <Combobox.GroupLabel className={styles.groupLabel}>{group.value}</Combobox.GroupLabel>}
                  <Combobox.Collection>{(item: PickerItem) => <PickerRow key={item.id} item={item} />}</Combobox.Collection>
                </Combobox.Group>
              )
            : (item: PickerItem) => <PickerRow key={item.id} item={item} />}
        </Combobox.List>
        <Combobox.Empty className={styles.empty}>{emptyText ?? "Nichts gefunden. Ändere die Suche oder den Filter."}</Combobox.Empty>
      </Combobox.Root>
      {footer && <div className={styles.footer}>{footer}</div>}
    </div>
  );
}

/**
 * Choose one item from a long list: a dialog with a search field that narrows the list on every key. The first match is
 * highlighted, the arrow keys move, Enter picks it, Escape closes without picking.
 */
export function Picker({ open, onOpenChange, title, ...body }: PickerProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={title}>
      <PickerBody onOpenChange={onOpenChange} {...body} />
    </Dialog>
  );
}
