import { useTranslation } from "react-i18next";
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

/** The catalog key of an entry's name (`atlas.<name>` in `ui`); the names themselves are read when the atlas renders. */
type Name = "start" | "world" | "stories" | "pokemon" | "items" | "texts" | "technical" | "help" | "settings";
type Entry = { area: AreaId; name: Name; icon: IconName };
type Look = { name: Name; icon: IconName };

const MAIN: readonly Entry[] = [
  { area: "start", name: "start", icon: "house" },
  { area: "welt", name: "world", icon: "world" },
  { area: "geschichten", name: "stories", icon: "stories" },
  { area: "pokemon", name: "pokemon", icon: "pokemon" },
  { area: "gegenstaende", name: "items", icon: "items" },
  { area: "texte", name: "texts", icon: "texts" },
  { area: "technik", name: "technical", icon: "tech" },
];

const HELP: Look = { name: "help", icon: "help" };

const SETTINGS: Entry = { area: "einstellungen", name: "settings", icon: "settings" };

/** The column of areas on the left (88 px with labels, 56 px with symbols only); the active one has its area colour and a bar at the edge. */
export function Atlas({ area, onArea, locked, lockedHint, onHelp, density }: AtlasProps) {
  const { t } = useTranslation("ui");
  const compact = density === "kompakt";
  const content = (look: Look, active: boolean) => (
    <>
      <Icon name={look.icon} weight={active ? "fill" : "regular"} />
      {!compact && <span>{t(`atlas.${look.name}`)}</span>}
    </>
  );
  const entry = (e: Entry) => {
    const label = t(`atlas.${e.name}`);
    const isLocked = locked.includes(e.area);
    const button = (
      <button
        key={e.area}
        type="button"
        className={styles.entry}
        data-area={e.area}
        aria-label={label}
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
      <Tooltip key={e.area} content={label}>
        {button}
      </Tooltip>
    ) : (
      button
    );
  };
  const help = (
    <button type="button" className={styles.entry} aria-label={t("atlas.help")} onClick={onHelp}>
      {content(HELP, false)}
    </button>
  );
  return (
    <nav className={styles.atlas} aria-label={t("atlas.label")} data-dichte={density}>
      <div className={styles.group}>{MAIN.map(entry)}</div>
      <div className={styles.group}>
        {compact ? <Tooltip content={t("atlas.help")}>{help}</Tooltip> : help}
        {entry(SETTINGS)}
      </div>
    </nav>
  );
}
