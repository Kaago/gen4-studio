import { ContextMenu } from "@base-ui/react/context-menu";
import { Menu as BaseMenu } from "@base-ui/react/menu";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import type { RowDataAttrs } from "./ListRow";
import { MenuEntries } from "./Menu";
import type { MenuEntry } from "./Menu";
import menuStyles from "./Menu.module.css";
import styles from "./Reference.module.css";

export type Area = "welt" | "geschichten" | "pokemon" | "gegenstaende" | "texte" | "technik";

export const areaIcon: Record<Area, IconName> = {
  welt: "world",
  geschichten: "stories",
  pokemon: "pokemon",
  gegenstaende: "items",
  texte: "texts",
  technik: "tech",
};

export type ReferenceProps = {
  area: Area;
  label: string;
  /** Technical name or number, in mono after the label. */
  tech?: string;
  /** Opens the target, or its picker where the target is a choice (a wizard's chosen header). */
  onOpen: () => void;
  /** Where else the target is used; a right click lists them. An empty list says "Nirgends verwendet". */
  usedIn?: { label: string; onOpen: () => void }[];
  /** The target cannot be opened right now (e.g. its tool is not ready); the chip stays visible but inert. */
  disabled?: boolean;
  /** Pass-through `data-*` attributes of the chip (`data-header`, …) that tests and scripts address it by. */
  dataAttrs?: RowDataAttrs;
};

/** A chip in the color of the target's area that opens the target or its picker. */
export function Reference({ area, label, tech, onOpen, usedIn, disabled, dataAttrs }: ReferenceProps) {
  const chip = (
    <button {...dataAttrs} type="button" className={`${styles.chip} ${styles[area]}`} onClick={onOpen} disabled={disabled}>
      <Icon name={areaIcon[area]} />
      <span className={styles.label}>{label}</span>
      {tech && <span className={styles.tech}>{tech}</span>}
    </button>
  );
  if (!usedIn) return chip;
  const entries: MenuEntry[] =
    usedIn.length > 0 ? usedIn.map((u) => ({ label: u.label, onSelect: u.onOpen })) : [{ label: "Nirgends verwendet", onSelect: () => {}, disabled: true }];
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={<span className={styles.wrap} />}>{chip}</ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className={menuStyles.positioner}>
          <BaseMenu.Popup className={menuStyles.popup}>
            <BaseMenu.Group>
              <BaseMenu.GroupLabel className={menuStyles.groupLabel}>Verwendet in</BaseMenu.GroupLabel>
              <MenuEntries items={entries} />
            </BaseMenu.Group>
          </BaseMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}
