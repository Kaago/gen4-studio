import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, machineTable } from "../../api";
import { Button, Notice, ToggleGrid } from "../../ui/primitives";
import type { ToggleCell } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { useDataArea } from "../useDataArea";
import { ChoicePicker, useChoices } from "./pickers";
import type { Editing } from "./PokedexHand";
import { MACHINE_COUNT, machineLabel, moveName } from "./pokedexModel";
import styles from "./Pokedex.module.css";

interface Machines {
  /** The move of each machine; null while unread, when the game's table is not available, or when it could not be read. */
  moves: readonly number[] | null;
  /** Why the table could not be read (a null answer is no error: the table is not available for this game). */
  error: string | null;
}

const UNREAD: Machines = { moves: null, error: null };

/** The moves of TM01 to TM92 and HM01 to HM08 from arm9, read when the tab shows; without them the grid shows bare numbers. */
function useMachines(): Machines {
  const { project } = useDataArea("pokemon");
  const [read, setRead] = useState<(Machines & { project: unknown }) | null>(null);
  useEffect(() => {
    if (project === null) return;
    let current = true;
    machineTable().then(
      (moves) => current && setRead({ project, moves, error: null }),
      (e: unknown) => current && setRead({ project, moves: null, error: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }, [project]);
  return read !== null && read.project === project ? read : UNREAD;
}

/**
 * TM/VM: which machines the species can learn, as a grid of 92 TMs and 8 HMs named by their move ("TM40 Aero-Ass"), and "Von anderer Art
 * übernehmen …", which copies the machines and the learnset of another species in one undo step.
 */
export function MachinesTab({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const { rows, labelOf } = useDataArea("pokemon");
  const { moves: machines, error } = useMachines();
  const [copying, setCopying] = useState(false);
  const self = e.index;
  const others = useCallback((index: number) => index !== self, [self]);
  const species = useChoices("species", copying, others);
  const tm = e.doc.tm;
  const on = tm.filter(Boolean).length;
  const cells = useMemo<ToggleCell[]>(
    () =>
      Array.from({ length: MACHINE_COUNT }, (_, n) => {
        const move = machines?.[n];
        return { id: String(n), label: machineLabel(n), name: move === undefined ? undefined : moveName(e.names.moves, move), checked: tm[n] ?? false, disabled: e.locked.personal };
      }),
    [machines, tm, e.names.moves, e.locked.personal, t],
  );
  const toggle = (id: string, checked: boolean) => {
    const n = Number(id);
    void e.edit({ Tm: { n, on: checked } }, t(checked ? "machinesTab.toggleOn" : "machinesTab.toggleOff", { name: e.name, machine: machineLabel(n) }));
  };
  const copyFrom = (from: number) => {
    const row = rows.find((r) => r.index === from);
    const source = row ? labelOf(row) : t("speciesFallback", { id: id4(from) });
    void e.edit({ CopyMoves: { from } }, t("machinesTab.copyLabel", { name: e.name, source }));
  };
  return (
    <div className={styles.tableTab}>
      <div className={styles.actions}>
        <span className={styles.count}>{t("machinesTab.count", { on, total: MACHINE_COUNT })}</span>
        <Button kind="quiet" size="s" data-copy-moves="" disabled={e.locked.personal || e.locked.learnset} onClick={() => setCopying(true)}>
          {t("machinesTab.copyButton")}
        </Button>
      </div>
      {error !== null && <Notice kind="error">{t("machinesTab.unreadable", { error })}</Notice>}
      <ToggleGrid ariaLabel={t("machinesTab.gridLabel")} cells={cells} columns={10} onToggle={toggle} />
      <ChoicePicker open={copying} onOpenChange={setCopying} title={t("machinesTab.copyTitle")} items={species} onPick={copyFrom} footer={t("machinesTab.copyFooter", { name: e.name })} />
    </div>
  );
}
