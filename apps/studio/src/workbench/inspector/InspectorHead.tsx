import type { EventKind } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { Badge, Icon, Menu } from "../../ui/primitives";
import type { IconName, MenuEntry } from "../../ui/primitives";
import styles from "./InspectorHead.module.css";

/** What the inspector shows, as the head names it. */
export type InspectorKind = "figure" | "door" | "trigger" | "sign" | "building" | "map" | "area" | "room" | "place" | "look";

/**
 * The word of each kind in the eyebrow and its symbol. Bauen speaks of the game; Tüfteln says "Header" for an area, a room or a place and
 * "Area-Data" for the look (`tinker`), the other words are the same in both depths.
 */
const KINDS: Record<InspectorKind, { word: string; tinker?: string; icon: IconName }> = {
  figure: { word: "Figur", icon: "figure" },
  door: { word: "Tür", icon: "door" },
  trigger: { word: "Auslöser", icon: "trigger" },
  sign: { word: "Schild", icon: "sign" },
  building: { word: "Gebäude", icon: "building" },
  map: { word: "Karte", icon: "map" },
  area: { word: "Gebiet", tinker: "Header", icon: "outside" },
  room: { word: "Raum", tinker: "Header", icon: "room" },
  /** A header that is neither an area nor a room in the world tree (the landscape, a header without maps). */
  place: { word: "Ort", tinker: "Header", icon: "place" },
  look: { word: "Aussehen", tinker: "Area-Data", icon: "look" },
};

/** The kind of an event class: a warp is the door, an object the figure, a trigger the trigger, a background event the sign. */
export const EVENT_INSPECTOR_KIND: Record<EventKind, InspectorKind> = { warp: "door", object: "figure", coord: "trigger", bg: "sign" };

export interface InspectorHeadProps {
  kind: InspectorKind;
  title: string;
  /** The record in the words of the ROM, grey and in mono below the title ("Objekt 2 · Eventdatei 0390"). */
  ident?: string;
  /** The record has changes that are not saved yet. */
  unsaved?: boolean;
  /** The record cannot be changed (a read-only event file). */
  locked?: boolean;
  /** A change of the record cannot be undone. */
  irreversible?: boolean;
  /** What can be done with the record: the menu „Aktionen“ at the right end, left out when there is nothing. */
  actions?: MenuEntry[];
  /** Bauen, or Tüfteln: the word of an area, a room and the look. */
  depth: Depth;
}

/** The head every inspector view shares: the kind as an eyebrow with its symbol, the title, the ident, the badges and the actions. */
export function InspectorHead({ kind, title, ident, unsaved, locked, irreversible, actions, depth }: InspectorHeadProps) {
  const { word, tinker, icon } = KINDS[kind];
  const badges = unsaved || locked || irreversible;
  return (
    <header className={styles.head} data-inspector-kind={kind}>
      <div className={styles.text}>
        <span className={styles.eyebrow} data-eyebrow="">
          <Icon name={icon} />
          {depth === "tuefteln" && tinker !== undefined ? tinker : word}
        </span>
        <h2 className={styles.title}>{title}</h2>
        {ident && (
          <span className={styles.ident} data-ident="">
            {ident}
          </span>
        )}
      </div>
      {badges && (
        <div className={styles.badges}>
          {unsaved && <Badge kind="unsaved" />}
          {locked && <Badge kind="locked" />}
          {irreversible && <Badge kind="irreversible" />}
        </div>
      )}
      {actions && actions.length > 0 && (
        <div className={styles.actions}>
          <Menu label="Aktionen" trigger={<Icon name="more" />} items={actions} />
        </div>
      )}
    </header>
  );
}
