import { useMemo } from "react";
import { Picker } from "../primitives";
import type { PickerItem } from "../primitives";

/** The groups of the palette in the order it lists them; the entries name theirs with these words. */
export const PALETTE_GROUPS = ["Befehle", "Orte", "Header", "Textarchive", "Pokémon", "Trainer", "Begegnungen", "Attacken", "Gegenstände", "Scripts"] as const;
export type PaletteGroup = (typeof PALETTE_GROUPS)[number];

/** With an empty search the palette lists the commands only. Module constants: the picker memoises on them. */
const EMPTY_QUERY_GROUPS: readonly string[] = ["Befehle"];

/** At most this many hits per group (spec §5.8). */
export const PALETTE_LIMIT = 8;

export type PaletteProps<T extends PickerItem> = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Every entry the palette can find (commands, places, headers, archives, scripts), each with its `group`. */
  items: readonly T[];
  /** Runs the chosen entry; called after the palette closed, never for a disabled one. */
  onRun: (item: T) => void;
};

/**
 * The command palette (⌘K, spec §5.8): the picker in its palette form, top centre, 640 wide. Typing filters at once over
 * every group; with nothing typed only the commands show. Enter runs the highlighted entry after the palette closed (so
 * whatever it opens gets the focus), Escape closes it.
 */
export function Palette<T extends PickerItem>({ open, onOpenChange, items, onRun }: PaletteProps<T>) {
  // The picker hands back a `PickerItem`; the entry of that id is the caller's own type.
  const byId = useMemo(() => new Map(items.map((item) => [item.id, item])), [items]);
  return (
    <Picker
      variant="palette"
      open={open}
      onOpenChange={onOpenChange}
      title="Befehlspalette"
      placeholder="Suchen oder Befehl eingeben"
      emptyText="Nichts gefunden. Ändere die Suche."
      items={items}
      limitPerGroup={PALETTE_LIMIT}
      groupOrder={PALETTE_GROUPS}
      emptyQueryGroups={EMPTY_QUERY_GROUPS}
      onPick={(picked) => {
        const item = byId.get(picked.id);
        if (item && !item.disabled) onRun(item);
      }}
    />
  );
}
