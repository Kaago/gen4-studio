import { Combobox } from "@base-ui/react/combobox";
import { useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Dialog } from "./Dialog";
import { Icon } from "./Icon";
import { Kbd } from "./Kbd";
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
  /**
   * `dialog` (default) is the centred picker. `palette` is the command palette: anchored near the top, 640 wide, title for
   * screen readers only, no close button, the typed text marked in each row, a footer with the key hints and the hit
   * count; `onPick` is then called on the tick after the close request, so the focus return of the dialog cannot run
   * into whatever the pick opens.
   */
  variant?: "dialog" | "palette";
  /** At most this many rows per group, cut after filtering; the hit count in the palette footer is taken before the cut. */
  limitPerGroup?: number;
  /** Groups in this order first; groups not named follow in order of first appearance. */
  groupOrder?: readonly string[];
  /** With an empty search only these groups are listed (e.g. the commands); typing searches all groups. */
  emptyQueryGroups?: readonly string[];
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

/** Groups named in `order` first, in that order; the others after them, order kept. */
function orderGroups(groups: Group[], order: readonly string[]): Group[] {
  const rank = (g: Group) => {
    const at = order.indexOf(g.value);
    return at === -1 ? order.length : at;
  };
  // Array.prototype.sort is stable: groups with the same rank keep their order of first appearance.
  return [...groups].sort((a, b) => rank(a) - rank(b));
}

/** `text` with the first occurrence of `query` (ignoring case) wrapped in `<mark>`. */
function highlight(text: string, query: string): ReactNode {
  const q = query.trim().toLowerCase();
  if (q === "") return text;
  const lower = text.toLowerCase();
  const at = lower.length === text.length ? lower.indexOf(q) : -1;
  if (at === -1) return text;
  return (
    <>
      {text.slice(0, at)}
      <mark className={styles.mark}>{text.slice(at, at + q.length)}</mark>
      {text.slice(at + q.length)}
    </>
  );
}

function PickerRow({ item, query }: { item: PickerItem; query: string | null }) {
  const icon = item.icon ?? (item.area ? areaIcon[item.area] : undefined);
  const mark = (text: string) => (query === null ? text : highlight(text, query));
  return (
    <Combobox.Item value={item} disabled={item.disabled} className={styles.row}>
      {item.preview && <span className={styles.preview}>{item.preview}</span>}
      {icon && (
        <span className={item.area ? `${styles.area} ${styles[item.area]}` : styles.area}>
          <Icon name={icon} />
        </span>
      )}
      {item.description ? (
        <span className={styles.text}>
          <span className={styles.label}>{mark(item.label)}</span>
          <span className={styles.description}>{mark(item.description)}</span>
        </span>
      ) : (
        <span className={styles.label}>{mark(item.label)}</span>
      )}
      {item.tech && <span className={styles.tech}>{item.tech}</span>}
      {item.count && <span className={styles.count}>{item.count}</span>}
      {item.shortcut && (
        <span className={styles.shortcut}>
          <Kbd keys={item.shortcut} />
        </span>
      )}
    </Combobox.Item>
  );
}

/** Mounted only while the dialog is open, so the query and the highlight start fresh every time. */
function PickerBody({
  onOpenChange,
  items,
  onPick,
  filters,
  filter,
  onFilter,
  placeholder,
  footer,
  emptyText,
  variant = "dialog",
  limitPerGroup,
  groupOrder,
  emptyQueryGroups,
}: Omit<PickerProps, "open" | "title">) {
  const palette = variant === "palette";
  const [query, setQuery] = useState("");
  // The body is mounted anew for every opening, so this guard resets with it. Base UI keeps the popup (and the focused
  // input) mounted while it closes; a second Enter or a key repeat in that time must not queue a second pick.
  const picked = useRef(false);
  const { groups, total } = useMemo(() => {
    let found = filterItems(items, query);
    if (emptyQueryGroups && query.trim() === "") found = found.filter((item) => emptyQueryGroups.includes(item.group ?? ""));
    let shown = groupItems(found);
    if (groupOrder) shown = orderGroups(shown, groupOrder);
    // The count is taken before the cut: "12 Treffer" even when only two per group are listed.
    if (limitPerGroup !== undefined) shown = shown.map((g) => ({ ...g, items: g.items.slice(0, limitPerGroup) }));
    return { groups: shown, total: found.length };
  }, [items, query, emptyQueryGroups, groupOrder, limitPerGroup]);
  const grouped = groups.some((g) => g.value !== "");
  const rowQuery = palette ? query : null;
  return (
    <div className={palette ? `${styles.picker} ${styles.paletteBody}` : styles.picker}>
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
          if (!item || item.disabled) return;
          if (palette) {
            if (picked.current) return;
            picked.current = true;
            onOpenChange(false);
            setTimeout(() => onPick(item), 0);
            return;
          }
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
                  <Combobox.Collection>{(item: PickerItem) => <PickerRow key={item.id} item={item} query={rowQuery} />}</Combobox.Collection>
                </Combobox.Group>
              )
            : (item: PickerItem) => <PickerRow key={item.id} item={item} query={rowQuery} />}
        </Combobox.List>
        <Combobox.Empty className={styles.empty}>{emptyText ?? "Nichts gefunden. Ändere die Suche oder den Filter."}</Combobox.Empty>
      </Combobox.Root>
      {palette && (
        <div className={`${styles.footer} ${styles.paletteFooter}`}>
          <span>↑↓ wählen, ↵ ausführen</span>
          <span>{total} Treffer</span>
          {footer}
        </div>
      )}
      {!palette && footer && <div className={styles.footer}>{footer}</div>}
    </div>
  );
}

/**
 * Choose one item from a long list: a dialog with a search field that narrows the list on every key. The first match is
 * highlighted, the arrow keys move, Enter picks it, Escape closes without picking.
 */
export function Picker({ open, onOpenChange, title, ...body }: PickerProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={title} variant={body.variant}>
      <PickerBody onOpenChange={onOpenChange} {...body} />
    </Dialog>
  );
}
