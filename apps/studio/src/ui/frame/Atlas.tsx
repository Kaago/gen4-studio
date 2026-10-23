import { Icon, Tooltip } from "../primitives";
import type { IconName } from "../primitives";
import type { Density } from "../tokens/theme";
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
  /** "kompakt" shows the symbols only (the shell's column narrows to match); every entry keeps its name as `aria-label`. */
  density: Density;
};

type Entry = { area: AreaId; label: string; icon: IconName };
type Look = { label: string; icon: IconName };

const MAIN: readonly Entry[] = [
  { area: "start", label: "Start", icon: "house" },
  { area: "welt", label: "Welt", icon: "world" },
  { area: "geschichten", label: "Geschichten", icon: "stories" },
  { area: "pokemon", label: "Pokémon", icon: "pokemon" },
  { area: "gegenstaende", label: "Gegenstände", icon: "items" },
  { area: "texte", label: "Texte", icon: "texts" },
  { area: "technik", label: "Technik", icon: "tech" },
];

const HELP: Look = { label: "Hilfe", icon: "help" };

const SETTINGS: Entry = { area: "einstellungen", label: "Einstellungen", icon: "settings" };

/** The column of areas on the left (88 px with labels, 56 px with symbols only); the active one has its area colour and a bar at the edge. */
export function Atlas({ area, onArea, locked, lockedHint, onHelp, density }: AtlasProps) {
  const compact = density === "kompakt";
  const content = (look: Look, active: boolean) => (
    <>
      <Icon name={look.icon} weight={active ? "fill" : "regular"} />
      {!compact && <span>{look.label}</span>}
    </>
  );
  const entry = (e: Entry) => {
    const isLocked = locked.includes(e.area);
    const button = (
      <button
        key={e.area}
        type="button"
        className={styles.entry}
        data-area={e.area}
        aria-label={e.label}
        aria-current={e.area === area ? "page" : undefined}
        aria-disabled={isLocked ? true : undefined}
        onClick={isLocked ? undefined : () => onArea(e.area)}
      >
        {content(e, e.area === area)}
      </button>
    );
    if (isLocked && lockedHint[e.area]) {
      return (
        <Tooltip key={e.area} content={lockedHint[e.area]}>
          {button}
        </Tooltip>
      );
    }
    // Without labels, hovering says the name.
    return compact ? (
      <Tooltip key={e.area} content={e.label}>
        {button}
      </Tooltip>
    ) : (
      button
    );
  };
  const help = (
    <button type="button" className={styles.entry} aria-label={HELP.label} onClick={onHelp}>
      {content(HELP, false)}
    </button>
  );
  return (
    <nav className={styles.atlas} aria-label="Bereiche" data-dichte={density}>
      <div className={styles.group}>{MAIN.map(entry)}</div>
      <div className={styles.group}>
        {compact ? <Tooltip content={HELP.label}>{help}</Tooltip> : help}
        {entry(SETTINGS)}
      </div>
    </nav>
  );
}
