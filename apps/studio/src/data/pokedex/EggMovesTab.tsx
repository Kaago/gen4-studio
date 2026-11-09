import { useEffect, useId, useMemo, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { EggMovesDto, MoveRowExtra } from "../../api";
import { Button, IconButton, Notice, Table, TypeBadge } from "../../ui/primitives";
import type { TableRow } from "../../ui/primitives";
import { useDataSnapshot } from "../dataStore";
import { useDataArea } from "../useDataArea";
import { ChoicePicker, PickField, useChoices } from "./pickers";
import type { Editing } from "./PokedexHand";
import { MAX_EGG_MOVES, moveName, spaced } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/** An egg move by what it is, not where it stands: the move and which entry of that move it is (a list may name a move twice). */
interface EggRef {
  move: number;
  nth: number;
}

function eggRefs(moves: readonly number[]): EggRef[] {
  const seen = new Map<number, number>();
  return moves.map((move) => {
    const nth = seen.get(move) ?? 0;
    seen.set(move, nth + 1);
    return { move, nth };
  });
}

const eggKey = (ref: EggRef): string => `m${ref.move}-${ref.nth}`;

/** Where entry `ref` stands in `moves`; -1 when it is gone. */
function findEgg(moves: readonly number[], ref: EggRef): number {
  let nth = 0;
  for (let i = 0; i < moves.length; i++) {
    if (moves[i] !== ref.move) continue;
    if (nth === ref.nth) return i;
    nth++;
  }
  return -1;
}

/** The table as an editable one: null where the game has none or it is locked. */
const editable = (table: EggMovesDto | null): EggMovesDto | null => (table === null || table.locked !== null ? null : table);

const countText = (t: TFunction<"data">, n: number) => t("eggMovesTab.count", { count: n });

/**
 * Why „Zeile hinzufügen“ is locked: 16 moves per species at most (`data_edit` refuses more), or Platinum's table in overlay 5 has no
 * room for another word (a species without moves needs two: its mark and the move). Null: a move can be added.
 */
function addBlocked(table: EggMovesDto, t: TFunction<"data">): string | null {
  const count = table.moves.length;
  if (count >= MAX_EGG_MOVES) return t("eggMovesTab.maxPerSpecies", { max: MAX_EGG_MOVES });
  const budget = table.budget;
  if (budget !== null && budget.used + (count > 0 ? 1 : 2) > budget.total) return t("eggMovesTab.tableFull");
  return null;
}

/**
 * Ei-Attacken: the moves a species passes on through an egg, as a table (the move from a picker, a button to remove the row) in the
 * game's order. The table belongs to every species: HeartGold/SoulSilver keep it in a NARC, Platinum in overlay 5 with a fixed budget
 * of words (swapping always goes, adding only against a removal; the foot names how full it is). Every change writes the species' whole
 * list (`EggMoves`), built when its turn comes from the list the changes before it left. A table the studio cannot read safely is locked
 * with the kernel's reason and shows no fields; a game without a known table (Diamond/Pearl) says so. A refusal of the backend (table
 * full, the Platinum signature, more than 16 moves) shows above the tabs.
 */
export function EggMovesTab({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const columns = [
    { key: "move", label: t("pokedexTable.columns.move") },
    { key: "type", label: t("pokedexTable.columns.type"), width: 96 },
    { key: "remove", label: t("pokedexTable.columns.remove"), width: 88, align: "end" as const },
  ];
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
  const helpId = useId();

  const table = e.doc.egg_moves;
  const moves = table?.moves;
  const refs = useMemo(() => eggRefs(moves ?? []), [moves]);

  if (table === null) {
    return (
      <div className={styles.tableTab} data-tab="eggMoves">
        <Notice kind="info">{t("eggMovesTab.noTable")}</Notice>
      </div>
    );
  }
  if (table.locked !== null) {
    return (
      <div className={styles.tableTab} data-tab="eggMoves">
        <Notice kind="warning">{t("eggMovesTab.locked", { reason: table.locked })}</Notice>
      </div>
    );
  }

  const name = (id: number) => moveName(e.names.moves, id);
  const resized = (from: number, to: number) => t("eggMovesTab.resized", { name: e.name, from, to });
  const write = (make: (list: readonly number[]) => { list: number[]; label: string } | null) =>
    void e.update((doc) => {
      const now = editable(doc.egg_moves);
      const made = now === null ? null : make(now.moves);
      return made === null ? null : { patch: { EggMoves: { moves: made.list } }, label: made.label };
    });
  /** Writes what `make` does to entry `ref` of the list as it is when the write comes; nothing when the entry is gone. */
  const change = (ref: EggRef, make: (list: readonly number[], i: number) => { list: number[]; label: string } | null) =>
    write((list) => {
      const i = findEgg(list, ref);
      return i < 0 ? null : make(list, i);
    });

  const setMove = (ref: EggRef, move: number) =>
    change(ref, (list, i) => (move === ref.move ? null : { list: list.map((m, j) => (j === i ? move : m)), label: t("eggMovesTab.swapped", { name: e.name }) }));
  const remove = (ref: EggRef) => change(ref, (list, i) => ({ list: list.filter((_, j) => j !== i), label: resized(list.length, list.length - 1) }));
  const add = (move: number) => write((list) => (list.length >= MAX_EGG_MOVES ? null : { list: [...list, move], label: resized(list.length, list.length + 1) }));

  const count = table.moves.length;
  const blocked = addBlocked(table, t);
  const budget = table.budget;
  const footer =
    budget === null ? countText(t, count) : <span data-egg-budget="">{t("eggMovesTab.footerBudget", { counted: countText(t, count), used: spaced(budget.used), total: spaced(budget.total) })}</span>;

  const rows: TableRow[] = table.moves.map((id, i) => {
    const ref = refs[i]!;
    const move = name(id);
    const type = typeOf.get(id);
    return {
      key: eggKey(ref),
      attrs: { "data-egg": String(i), "data-move": String(id) },
      cells: {
        move: <PickField label={t("pokedexTable.moveInRow", { row: i + 1 })} text={move} choice="move" onPick={(v) => setMove(ref, v)} />,
        type: type === undefined ? null : <TypeBadge type={type} size="s" />,
        remove: <IconButton label={t("eggMovesTab.removeMove", { move })} icon="close" onClick={() => remove(ref)} />,
      },
    };
  });

  return (
    <div className={styles.tableTab} data-tab="eggMoves">
      <div className={styles.actions}>
        <Button
          kind="secondary"
          size="s"
          icon="plus"
          data-add-egg=""
          aria-disabled={blocked !== null || undefined}
          aria-describedby={blocked !== null ? helpId : undefined}
          onClick={() => {
            if (blocked === null) setAdding(true);
          }}
        >
          {t("pokedexTable.addRow")}
        </Button>
        {blocked !== null && (
          <span id={helpId} className={styles.note}>
            {blocked}
          </span>
        )}
      </div>
      <Table
        ariaLabel={t("eggMovesTab.tableLabel", { name: e.name })}
        columns={columns}
        rows={rows}
        selected={selected}
        onSelect={setSelected}
        onDeleteRow={(key) => {
          const ref = refs.find((r) => eggKey(r) === key);
          if (ref) remove(ref);
        }}
        footer={footer}
      />
      <ChoicePicker open={adding} onOpenChange={setAdding} title={t("eggMovesTab.addTitle")} items={addChoices} onPick={add} footer={t("eggMovesTab.addFooter")} />
    </div>
  );
}
