import type { TFunction } from "i18next";
import type { ComponentType, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { DataRowDto } from "../api";
import { ListPane } from "../ui/frame/ListPane";
import { useShell } from "../ui/frame/ShellContext";
import { depthAtLeast, viewDepth } from "../ui/frame/shellState";
import { EmptyState, PanelHead } from "../ui/primitives";
import { areaLabel, subLabel } from "./dataModel";
import type { DataArea, DataSub, ListKind } from "./dataModel";
import { useDataArea } from "./useDataArea";
import styles from "./DataArea.module.css";

/**
 * What a part of a data area brings: its list (in the area's list column), its view (Schau) and its editor (Hand), and for Erzählen
 * optionally a view and an editor of their own; without, Erzählen shows the view and the editor of Bauen.
 */
export interface DataPart {
  List: ComponentType;
  View: ComponentType;
  Hand: ComponentType;
  /** The view in Erzählen; without, `View`. */
  ViewNarrate?: ComponentType;
  /** The editor in Erzählen; without, `Hand`. */
  HandNarrate?: ComponentType;
  /** What the list head says the list holds ("493 Arten, 12 Formen"); without, the rows counted ("471 Attacken"). */
  count?: (rows: readonly DataRowDto[]) => string;
}

/** What the view and the hand say while nothing of a part is chosen; `first`: the view's button that chooses the first member. */
function nothingChosen(sub: DataSub, t: TFunction<"data">): { what: string; why: string; first: string } {
  return { what: t(`dataParts.nothing.${sub}.what`), why: t(`dataParts.nothing.${sub}.why`), first: t(`dataParts.nothing.${sub}.first`) };
}

/** What the area's lists hold, for the pane's "508 Arten" and the empty project. */
const count = (t: TFunction<"data">, kind: ListKind, n: number) => t(`dataParts.noun.${kind}`, { count: n });

/** What stands in place of a part while it cannot show (no project, the game is locked, the list is not read or not readable); null: it can. */
function useBlocked(area: DataArea): ReactNode | null {
  const { t } = useTranslation("data");
  const { project, locked, error, loading } = useDataArea(area);
  if (project === null) return <EmptyState what={t("dataParts.noProject")} why={t(`dataParts.noProjectWhy.${area}`)} />;
  if (locked !== null) return <EmptyState what={t("dataParts.lockedForGame")} why={locked} />;
  if (error !== null) return <EmptyState what={t("dataParts.listUnreadable")} why={error} />;
  if (loading) return <p className={styles.note}>{t("loading")}</p>;
  return null;
}

/**
 * The list column of a data area: the head (with the switcher of the parts in Pokémon, the count of the list), and the list of the part
 * that shows; in its place why it cannot show.
 */
export function DataListPane({ area, parts, open, onToggle }: { area: DataArea; parts: Partial<Record<DataSub, DataPart>>; open: boolean; onToggle: () => void }) {
  const { t } = useTranslation("data");
  const { sub, switcher, kind, rows } = useDataArea(area);
  const blocked = useBlocked(area);
  const List = parts[sub]?.List;
  const counted = parts[sub]?.count;
  const subtitle = kind !== null && !blocked && rows.length > 0 ? (counted ? counted(rows) : count(t, kind, rows.length)) : undefined;
  return (
    <ListPane title={areaLabel(area)} subtitle={subtitle} switcher={switcher ?? undefined} open={open} onToggle={onToggle}>
      {blocked ?? (List ? <List /> : <EmptyState what={subLabel(sub)} why={nothingChosen(sub, t).why} />)}
    </ListPane>
  );
}

/**
 * The view or the hand of a data area: the part's own (in Erzählen its narrating one, if it has one), once something is chosen; before
 * that, what to do, and in the view a button that shows the first member (the first above 0: member 0 is the games' empty entry, "-----",
 * "Keines").
 */
export function DataPanel({ area, parts, place }: { area: DataArea; parts: Partial<Record<DataSub, DataPart>>; place: "View" | "Hand" }) {
  const { t } = useTranslation("data");
  const { sub, row, rows, select } = useDataArea(area);
  const blocked = useBlocked(area);
  // The depth of this area, not of the one in front: a deck kept behind another area keeps its components.
  const narrating = !depthAtLeast(viewDepth(area, useShell().state.depth), "bauen");
  const part = parts[sub];
  const Part = (narrating ? (place === "View" ? part?.ViewNarrate : part?.HandNarrate) : undefined) ?? part?.[place];
  const nothing = nothingChosen(sub, t);
  const first = rows.find((r) => r.index > 0) ?? rows[0];
  const action = place === "View" && nothing.first && first ? { label: nothing.first, onClick: () => select(first.index) } : undefined;
  return (
    <div className={place === "View" ? styles.view : styles.hand} data-data-panel={place === "View" ? "view" : "hand"}>
      {blocked !== null ? (
        <div className={styles.empty}>{blocked}</div>
      ) : row === null || !Part ? (
        <div className={styles.empty}>
          <EmptyState what={nothing.what} why={nothing.why} action={action} />
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
