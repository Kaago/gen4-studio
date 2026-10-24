import { useEffect, useMemo, useState } from "react";
import type { LearnEntryDto, MoveRowExtra } from "../../api";
import { Button, IconButton, NumberField, Table, TypeBadge } from "../../ui/primitives";
import type { TableRow } from "../../ui/primitives";
import { useDataSnapshot } from "../dataStore";
import { useDataArea } from "../useDataArea";
import { ChoicePicker, PickField, useChoices } from "./pickers";
import type { Editing } from "./PokedexHand";
import { MAX_LEARNSET, findLearn, learnKey, learnRefs, moveName, sortLearnset } from "./pokedexModel";
import type { LearnRef } from "./pokedexModel";
import styles from "./Pokedex.module.css";

const COLUMNS = [
  { key: "level", label: "Level", width: 112 },
  { key: "move", label: "Attacke" },
  { key: "type", label: "Typ", width: 96 },
  { key: "remove", label: "Entfernen", width: 88, align: "end" as const },
];

/**
 * Lernset: the level-up moves as a table (level 1 to 100, the move from a picker, a button to remove the row). The game reads 20 at most:
 * "Zeile hinzufügen" is locked from there. Every change writes the whole list, sorted by level (equal levels keep their order). A change
 * names its entry by move and occurrence and is built when its turn comes, from the list the changes before it left: a level typed and a
 * row removed right after both stay. Rows are keyed the same way, so the focus follows an entry a new level sorts elsewhere.
 */
export function LearnsetTab({ e }: { e: Editing }) {
  const { project, store } = useDataArea("pokemon");
  const snapshot = useDataSnapshot(store);
  useEffect(() => {
    if (project !== null) store.need(project, "move");
  }, [project, store]);
  const moveRows = snapshot.project === project ? snapshot.rows.move : undefined;
  const typeOf = useMemo(() => new Map((moveRows ?? []).map((r) => [r.index, (r.extra as MoveRowExtra | null)?.type])), [moveRows]);
  const [adding, setAdding] = useState(false);
  const addChoices = useChoices("move", adding);
  const [selected, setSelected] = useState<string | undefined>(undefined);

  const entries = e.doc.learnset;
  const refs = useMemo(() => learnRefs(entries), [entries]);
  const count = entries.length;
  const off = e.locked.learnset;
  const name = (id: number) => moveName(e.names.moves, id);
  const resized = (from: number, to: number) => `${e.name}: Lernset ${from} → ${to} Attacken`;

  /** Writes what `make` does to the entry `ref` of the list as it is when the write comes; nothing when the entry is gone. */
  const change = (ref: LearnRef, make: (list: readonly LearnEntryDto[], i: number) => { list: LearnEntryDto[]; label: string } | null) =>
    void e.update((doc) => {
      const i = findLearn(doc.learnset, ref);
      const made = i < 0 ? null : make(doc.learnset, i);
      return made === null ? null : { patch: { Learnset: { entries: sortLearnset(made.list) } }, label: made.label };
    });

  const setLevel = (ref: LearnRef, level: number) =>
    change(ref, (list, i) =>
      list[i]!.level === level ? null : { list: list.map((x, j) => (j === i ? { ...x, level } : x)), label: `${e.name}: ${name(ref.move_id)} Level ${list[i]!.level} → ${level}` },
    );
  const setMove = (ref: LearnRef, move: number) =>
    change(ref, (list, i) => (move === ref.move_id ? null : { list: list.map((x, j) => (j === i ? { ...x, move_id: move } : x)), label: `${e.name}: Lernset ${name(ref.move_id)} → ${name(move)}` }));
  const remove = (ref: LearnRef) => change(ref, (list, i) => ({ list: list.filter((_, j) => j !== i), label: resized(list.length, list.length - 1) }));
  const add = (move: number) =>
    void e.update((doc) =>
      doc.learnset.length >= MAX_LEARNSET
        ? null
        : { patch: { Learnset: { entries: sortLearnset([...doc.learnset, { move_id: move, level: 1 }]) } }, label: resized(doc.learnset.length, doc.learnset.length + 1) },
    );

  const rows: TableRow[] = entries.map((entry, i) => {
    const ref = refs[i]!;
    const move = name(entry.move_id);
    const type = typeOf.get(entry.move_id);
    return {
      key: learnKey(ref),
      attrs: { "data-learn": String(i), "data-move": String(entry.move_id) },
      disabled: off,
      cells: {
        level: <NumberField label={`Level von ${move}`} value={entry.level} min={1} max={100} onCommit={(v) => setLevel(ref, v)} disabled={off} showBase={e.showBase} />,
        move: <PickField label={`Attacke in Zeile ${i + 1}`} text={move} choice="move" onPick={(v) => setMove(ref, v)} disabled={off} />,
        type: type === undefined ? null : <TypeBadge type={type} size="s" />,
        remove: <IconButton label={`${move} auf Level ${entry.level} entfernen`} icon="close" disabled={off} onClick={() => remove(ref)} />,
      },
    };
  });

  return (
    <div className={styles.tableTab}>
      <div className={styles.actions}>
        <Button kind="secondary" size="s" icon="plus" data-add-learn="" disabled={off || count >= MAX_LEARNSET} onClick={() => setAdding(true)}>
          Zeile hinzufügen
        </Button>
        {count >= MAX_LEARNSET && <span className={styles.note}>Das Spiel liest höchstens 20 Attacken.</span>}
      </div>
      <Table
        ariaLabel={`Lernset von ${e.name}`}
        columns={COLUMNS}
        rows={rows}
        selected={selected}
        onSelect={setSelected}
        onDeleteRow={
          off
            ? undefined
            : (key) => {
                const ref = refs.find((r) => learnKey(r) === key);
                if (ref) remove(ref);
              }
        }
        footer={`${count} von ${MAX_LEARNSET}`}
      />
      <ChoicePicker open={adding} onOpenChange={setAdding} title="Attacke hinzufügen" items={addChoices} onPick={add} footer="Lernt sie auf Level 1; das Level lässt sich danach ändern." />
    </div>
  );
}
