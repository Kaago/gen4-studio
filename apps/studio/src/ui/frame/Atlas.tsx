import { Icon, Tooltip } from "../primitives";
import type { IconName } from "../primitives";
import type { AreaId } from "./shellState";
import styles from "./Atlas.module.css";

export type AtlasProps = {
  area: AreaId;
  onArea: (area: AreaId) => void;
  /** The areas that cannot be opened yet: `aria-disabled`, no click, the hint as tooltip. */
  locked: readonly AreaId[];
  lockedHint: Partial<Record<AreaId, string>>;
  /** Help is not an area: it opens a dialog. */
  onHelp: () => void;
};

type Entry = { area: AreaId; label: string; icon: IconName };

const MAIN: readonly Entry[] = [
  { area: "start", label: "Start", icon: "house" },
  { area: "welt", label: "Welt", icon: "world" },
  { area: "geschichten", label: "Geschichten", icon: "stories" },
  { area: "pokemon", label: "Pokémon", icon: "pokemon" },
  { area: "gegenstaende", label: "Gegenstände", icon: "items" },
  { area: "texte", label: "Texte", icon: "texts" },
  { area: "technik", label: "Technik", icon: "tech" },
];

const SETTINGS: Entry = { area: "einstellungen", label: "Einstellungen", icon: "settings" };

/** The 88 px column of areas on the left; the active one has its area colour and a bar at the edge. */
export function Atlas({ area, onArea, locked, lockedHint, onHelp }: AtlasProps) {
  const entry = (e: Entry) => {
    const isLocked = locked.includes(e.area);
    const button = (
      <button
        key={e.area}
        type="button"
        className={styles.entry}
        data-area={e.area}
        aria-current={e.area === area ? "page" : undefined}
        aria-disabled={isLocked ? true : undefined}
        onClick={isLocked ? undefined : () => onArea(e.area)}
      >
        <Icon name={e.icon} weight={e.area === area ? "fill" : "regular"} />
        <span>{e.label}</span>
      </button>
    );
    return isLocked && lockedHint[e.area] ? (
      <Tooltip key={e.area} content={lockedHint[e.area]}>
        {button}
      </Tooltip>
    ) : (
      button
    );
  };
  return (
    <nav className={styles.atlas} aria-label="Bereiche">
      <div className={styles.group}>{MAIN.map(entry)}</div>
      <div className={styles.group}>
        <button type="button" className={styles.entry} onClick={onHelp}>
          <Icon name="help" />
          <span>Hilfe</span>
        </button>
        {entry(SETTINGS)}
      </div>
    </nav>
  );
}
