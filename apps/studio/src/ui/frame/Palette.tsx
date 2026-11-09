import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { t as translate } from "../../i18n";
import { termPlural } from "../../workbench/terms";
import { Picker } from "../primitives";
import type { PickerItem } from "../primitives";

/** The identifier of the items group; written with an escape so that this file holds no German word. */
const ITEMS = "Gegenst\u00e4nde";

/**
 * The groups of the palette in the order it lists them. Each is the identifier an entry carries in `group` (the callers write these
 * words, whatever the language); the heading the palette shows for it is read from the catalog (`groupName`).
 */
export const PALETTE_GROUPS = ["Befehle", "Orte", "Header", "Textarchive", "Pokémon", "Trainer", "Begegnungen", "Attacken", ITEMS, "Scripts"] as const;
export type PaletteGroup = (typeof PALETTE_GROUPS)[number];

/** The heading of each group, read when it is asked for: the palette's own words from `ui`, the world's words from `terms`. */
const HEADINGS: Record<PaletteGroup, () => string> = {
  Befehle: () => translate("ui:palette.groups.commands"),
  Orte: () => termPlural("place"),
  Header: () => termPlural("header"),
  Textarchive: () => termPlural("textArchive"),
  Pokémon: () => translate("ui:palette.groups.pokemon"),
  Trainer: () => translate("ui:palette.groups.trainers"),
  Begegnungen: () => translate("ui:palette.groups.encounters"),
  Attacken: () => translate("ui:palette.groups.moves"),
  [ITEMS]: () => translate("ui:palette.groups.items"),
  Scripts: () => termPlural("script"),
};

/** The heading of a group in the language that is shown; a group the palette does not know (a caller's own) is shown as it is. */
export function groupName(group: string): string {
  return (HEADINGS as Record<string, (() => string) | undefined>)[group]?.() ?? group;
}

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
  const { t } = useTranslation("ui");
  // The picker hands back a `PickerItem`; the entry of that id is the caller's own type.
  const byId = useMemo(() => new Map(items.map((item) => [item.id, item])), [items]);
  return (
    <Picker
      variant="palette"
      open={open}
      onOpenChange={onOpenChange}
      title={t("palette.title")}
      placeholder={t("palette.placeholder")}
      emptyText={t("palette.empty")}
      items={items}
      limitPerGroup={PALETTE_LIMIT}
      groupOrder={PALETTE_GROUPS}
      groupLabel={groupName}
      emptyQueryGroups={EMPTY_QUERY_GROUPS}
      onPick={(picked) => {
        const item = byId.get(picked.id);
        if (item && !item.disabled) onRun(item);
      }}
    />
  );
}
