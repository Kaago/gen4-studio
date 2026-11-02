import { useMemo, useState } from "react";
import { Switch, Tabs } from "../../ui/primitives";
import type { SelectOption, TabItem } from "../../ui/primitives";
import { ClassRow, Column, HandTop, HandWaiting, NameRow, TeamTab, Texts, setDouble, useTrainerEditing } from "./TrainerHand";
import type { Editing } from "./TrainerHand";
import { isDouble, namedClassOptions } from "./trainerModel";
import styles from "./Trainer.module.css";

type NarrateTab = "team" | "trainer";

/** Trainer in Erzählen: the class by name, the name, whether it fights double, and its battle texts. */
function NarrateTrainerTab({ e }: { e: Editing }) {
  const classes = useMemo<SelectOption<number>[]>(() => namedClassOptions(e.names.classes, e.doc.class), [e.names.classes, e.doc.class]);
  return (
    <div className={styles.columns}>
      <Column title="Trainer">
        <ClassRow e={e} options={classes} />
        <NameRow e={e} />
        <Switch label="Doppelkampf" checked={isDouble(e.doc.battle_type)} disabled={e.locked} onChange={(on) => setDouble(e, on)} />
      </Column>
      <Texts e={e} />
    </div>
  );
}

/**
 * The editor (Hand) of a trainer in Erzählen: the team as in Bauen, and a Trainer tab with only what tells the story (class, name, double
 * battle, battle texts). The AI, the trainer's items, the team format and the raw data stay in Bauen and Tüfteln. Every field writes one
 * undo step with the same labels as the hand of Bauen.
 */
export function TrainerHandNarrate() {
  const { row, selected, index, editing, error, editError, unsaved, title } = useTrainerEditing(false);
  const [tab, setTab] = useState<NarrateTab>("team");
  const count = editing?.doc.party.length;
  const items = useMemo<TabItem[]>(
    () => [
      { id: "team", label: "Team", count, unsaved },
      { id: "trainer", label: "Trainer", unsaved },
    ],
    [count, unsaved],
  );

  if (row === null || selected === null) return null;
  if (editing === null) return <HandWaiting error={error} />;
  return (
    <div className={styles.hand} data-trainer={index} data-trainer-narrate="">
      <HandTop e={editing} editError={editError} title={title} />
      <Tabs items={items} value={tab} onChange={(id) => setTab(id as NarrateTab)} ariaLabel={`${title} bearbeiten`}>
        <div className={styles.tabBody}>
          {tab === "team" && <TeamTab e={editing} />}
          {tab === "trainer" && <NarrateTrainerTab e={editing} />}
        </div>
      </Tabs>
    </div>
  );
}
