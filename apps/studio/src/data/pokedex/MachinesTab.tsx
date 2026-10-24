import { useEffect, useMemo, useState } from "react";
import { machineTable } from "../../api";
import { Button, ToggleGrid } from "../../ui/primitives";
import type { ToggleCell } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import { ChoicePicker, useChoices } from "./pickers";
import type { Editing } from "./PokedexHand";
import { MACHINE_COUNT, id4, machineLabel, moveName } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/**
 * The moves of TM01 to TM92 and HM01 to HM08 from arm9, read once per project; null while unread or when the game's table is not
 * available (the grid then shows bare numbers).
 */
function useMachines(): readonly number[] | null {
  const { project } = useDataArea("pokemon");
  const [machines, setMachines] = useState<{ project: unknown; moves: readonly number[] | null } | null>(null);
  useEffect(() => {
    if (project === null) return;
    let current = true;
    machineTable().then(
      (moves) => current && setMachines({ project, moves }),
      () => current && setMachines({ project, moves: null }),
    );
    return () => {
      current = false;
    };
  }, [project]);
  return machines !== null && machines.project === project ? machines.moves : null;
}

/**
 * TM/VM: which machines the species can learn, as a grid of 92 TMs and 8 HMs named by their move ("TM40 Aero-Ass"), and "Von anderer Art
 * übernehmen …", which copies the machines and the learnset of another species in one undo step.
 */
export function MachinesTab({ e }: { e: Editing }) {
  const { rows, labelOf } = useDataArea("pokemon");
  const machines = useMachines();
  const [copying, setCopying] = useState(false);
  const species = useChoices("species", copying, e.index);
  const tm = e.doc.tm;
  const on = tm.filter(Boolean).length;
  const cells = useMemo<ToggleCell[]>(
    () =>
      Array.from({ length: MACHINE_COUNT }, (_, n) => {
        const move = machines?.[n];
        return { id: String(n), label: machineLabel(n), name: move === undefined ? undefined : moveName(e.names.moves, move), checked: tm[n] ?? false, disabled: e.locked.personal };
      }),
    [machines, tm, e.names.moves, e.locked.personal],
  );
  const toggle = (id: string, checked: boolean) => {
    const n = Number(id);
    void e.edit({ Tm: { n, on: checked } }, `${e.name}: ${machineLabel(n)} ${checked ? "aus → an" : "an → aus"}`);
  };
  const copyFrom = (from: number) => {
    const row = rows.find((r) => r.index === from);
    const source = row ? labelOf(row) : `Art ${id4(from)}`;
    void e.edit({ CopyMoves: { from } }, `${e.name}: TM/VM und Lernset wie ${source}`);
  };
  return (
    <div className={styles.tableTab}>
      <div className={styles.actions}>
        <span className={styles.count}>{`${on} von ${MACHINE_COUNT}`}</span>
        <Button kind="quiet" size="s" data-copy-moves="" disabled={e.locked.personal || e.locked.learnset} onClick={() => setCopying(true)}>
          Von anderer Art übernehmen …
        </Button>
      </div>
      <ToggleGrid ariaLabel="TM und VM" cells={cells} columns={10} onToggle={toggle} />
      <ChoicePicker open={copying} onOpenChange={setCopying} title="Von anderer Art übernehmen" items={species} onPick={copyFrom} footer={`Ersetzt TM/VM und Lernset von ${e.name}`} />
    </div>
  );
}
