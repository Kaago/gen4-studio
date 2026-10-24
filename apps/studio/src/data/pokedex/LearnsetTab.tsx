import { useEffect, useMemo, useState } from "react";
import type { LearnEntryDto, MoveRowExtra } from "../../api";
import { Button, IconButton, NumberField, Table, TypeBadge } from "../../ui/primitives";
import type { TableRow } from "../../ui/primitives";
import { useDataSnapshot } from "../dataStore";
import { useDataArea } from "../useDataArea";
import { ChoicePicker, PickField, useChoices } from "./pickers";
import type { Editing } from "./PokedexHand";
import { MAX_LEARNSET, moveName, sortLearnset } from "./pokedexModel";
import styles from "./Pokedex.module.css";

const COLUMNS = [
  { key: "level", label: "Level", width: 112 },
  { key: "move", label: "Attacke" },
  { key: "type", label: "Typ", width: 96 },
  { key: "remove", label: "Entfernen", width: 88, align: "end" as const },
];

/**
 * Lernset: the level-up moves as a table (level 1 to 100, the move from a picker, a button to remove the row). The game reads 20 at most:
 * "Zeile hinzufügen" is locked from there. Every change writes the whole list, sorted by level (equal levels keep their order).
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
  const count = entries.length;
  const off = e.locked.learnset;
  const name = (id: number) => moveName(e.names.moves, id);
  const write = (next: LearnEntryDto[], label: string) => void e.edit({ Learnset: { entries: sortLearnset(next) } }, label);
  const resized = (to: number) => `${e.name}: Lernset ${count} → ${to} Attacken`;

  const setLevel = (i: number, level: number) =>
    write(
      entries.map((x, j) => (j === i ? { ...x, level } : x)),
      `${e.name}: ${name(entries[i]!.move_id)} Level ${entries[i]!.level} → ${level}`,
    );
  const setMove = (i: number, move: number) =>
    write(
      entries.map((x, j) => (j === i ? { ...x, move_id: move } : x)),
      `${e.name}: Lernset ${name(entries[i]!.move_id)} → ${name(move)}`,
    );
  const remove = (i: number) => write(entries.filter((_, j) => j !== i), resized(count - 1));
  const add = (move: number) => write([...entries, { move_id: move, level: 1 }], resized(count + 1));

  const rows: TableRow[] = entries.map((entry, i) => {
    const move = name(entry.move_id);
    const type = typeOf.get(entry.move_id);
    return {
      key: `r${i}`,
      attrs: { "data-learn": String(i), "data-move": String(entry.move_id) },
      disabled: off,
      cells: {
        level: <NumberField label={`Level von ${move}`} value={entry.level} min={1} max={100} onCommit={(v) => setLevel(i, v)} disabled={off} showBase={e.showBase} />,
        move: <PickField label={`Attacke in Zeile ${i + 1}`} text={move} choice="move" onPick={(v) => setMove(i, v)} disabled={off} />,
        type: type === undefined ? null : <TypeBadge type={type} size="s" />,
        remove: <IconButton label={`${move} auf Level ${entry.level} entfernen`} icon="close" disabled={off} onClick={() => remove(i)} />,
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
        onDeleteRow={off ? undefined : (key) => remove(Number(key.slice(1)))}
        footer={`${count} von ${MAX_LEARNSET}`}
      />
      <ChoicePicker open={adding} onOpenChange={setAdding} title="Attacke hinzufügen" items={addChoices} onPick={add} footer="Lernt sie auf Level 1; das Level lässt sich danach ändern." />
    </div>
  );
}
