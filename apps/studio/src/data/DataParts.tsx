import type { ComponentType, ReactNode } from "react";
import type { DataRowDto } from "../api";
import { ListPane } from "../ui/frame/ListPane";
import { EmptyState, PanelHead } from "../ui/primitives";
import { AREA_LABEL, SUB_LABEL, SUB_LOCKED } from "./dataModel";
import type { DataArea, DataSub, ListKind } from "./dataModel";
import { useDataArea } from "./useDataArea";
import styles from "./DataArea.module.css";

/** What a part of a data area brings: its list (in the area's list column), its view (Schau) and its editor (Hand). */
export interface DataPart {
  List: ComponentType;
  View: ComponentType;
  Hand: ComponentType;
  /** What the list head says the list holds ("493 Arten, 12 Formen"); without, the rows counted ("471 Attacken"). */
  count?: (rows: readonly DataRowDto[]) => string;
}

/** What the view and the hand say while nothing of a part is chosen. */
export const NOTHING_CHOSEN: Record<DataSub, { what: string; why: string }> = {
  pokedex: { what: "Kein Pokémon gewählt.", why: "Wähle links eine Art; hier erscheinen Werte und Fundorte." },
  moves: { what: "Keine Attacke gewählt.", why: "Wähle links eine Attacke; hier erscheinen ihre Werte und wer sie lernt." },
  items: { what: "Kein Gegenstand gewählt.", why: "Wähle links einen Gegenstand; hier erscheinen Preis, Tasche und Beschreibung." },
  trainer: { what: "Kein Trainer gewählt.", why: `Trainer ${SUB_LOCKED.trainer ?? ""}.` },
  encounters: { what: "Keine Begegnung gewählt.", why: `Begegnungen ${SUB_LOCKED.encounters ?? ""}.` },
};

/** What the area's lists hold, for the pane's "508 Arten" and the empty project. */
const NOUN: Record<ListKind, [string, string]> = { species: ["Art", "Arten"], move: ["Attacke", "Attacken"], item: ["Gegenstand", "Gegenstände"] };
const count = (kind: ListKind, n: number) => `${n} ${NOUN[kind][n === 1 ? 0 : 1]}`;

const NO_PROJECT: Record<DataArea, string> = {
  pokemon: "Öffne ein Projekt, dann stehen hier seine Pokémon und Attacken.",
  gegenstaende: "Öffne ein Projekt, dann stehen hier seine Gegenstände.",
};

/** What stands in place of a part while it cannot show (no project, the game is locked, the list is not read or not readable); null: it can. */
function useBlocked(area: DataArea): ReactNode | null {
  const { project, locked, error, loading } = useDataArea(area);
  if (project === null) return <EmptyState what="Kein Projekt geöffnet." why={NO_PROJECT[area]} />;
  if (locked !== null) return <EmptyState what="Für dieses Spiel gesperrt." why={locked} />;
  if (error !== null) return <EmptyState what="Die Liste ist nicht lesbar." why={error} />;
  if (loading) return <p className={styles.note}>Lädt …</p>;
  return null;
}

/**
 * The list column of a data area: the head (with the switcher of the parts in Pokémon, the count of the list), and the list of the part
 * that shows; in its place why it cannot show.
 */
export function DataListPane({ area, parts, open, onToggle }: { area: DataArea; parts: Partial<Record<DataSub, DataPart>>; open: boolean; onToggle: () => void }) {
  const { sub, switcher, kind, rows } = useDataArea(area);
  const blocked = useBlocked(area);
  const List = parts[sub]?.List;
  const counted = parts[sub]?.count;
  const subtitle = kind !== null && !blocked && rows.length > 0 ? (counted ? counted(rows) : count(kind, rows.length)) : undefined;
  return (
    <ListPane title={AREA_LABEL[area]} subtitle={subtitle} switcher={switcher ?? undefined} open={open} onToggle={onToggle}>
      {blocked ?? (List ? <List /> : <EmptyState what={SUB_LABEL[sub]} why={NOTHING_CHOSEN[sub].why} />)}
    </ListPane>
  );
}

/** The view or the hand of a data area: the part's own, once something is chosen; before that, what to do. */
export function DataPanel({ area, parts, place }: { area: DataArea; parts: Partial<Record<DataSub, DataPart>>; place: "View" | "Hand" }) {
  const { sub, row } = useDataArea(area);
  const blocked = useBlocked(area);
  const Part = parts[sub]?.[place];
  const nothing = NOTHING_CHOSEN[sub];
  return (
    <div className={place === "View" ? styles.view : styles.hand} data-data-panel={place === "View" ? "view" : "hand"}>
      {blocked !== null ? (
        <div className={styles.empty}>{blocked}</div>
      ) : row === null || !Part ? (
        <div className={styles.empty}>
          <EmptyState what={nothing.what} why={nothing.why} />
        </div>
      ) : (
        <Part />
      )}
    </div>
  );
}

/**
 * The stand-in of a part's view or editor until its own comes (Tasks 14 to 16 replace it in `POKEMON_PARTS` and `ITEM_PARTS`): the
 * head with the chosen member's name and number.
 */
export function chosenHead(area: DataArea) {
  return function ChosenHead() {
    const { row, labelOf } = useDataArea(area);
    if (row === null) return null;
    return <PanelHead title={labelOf(row)} subtitle={row.tech} />;
  };
}
