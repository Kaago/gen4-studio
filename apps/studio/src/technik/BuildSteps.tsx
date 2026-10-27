import { useState } from "react";
import type { BuildStep } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { Section, Table } from "../ui/primitives";
import type { TableColumn, TableRow } from "../ui/primitives";
// With its extension: on a case-insensitive disk (macOS) "./buildSteps" and "./BuildSteps" would both find buildSteps.ts first.
import { WATCH_PREFIX, formatSeconds, stepLabel } from "./buildSteps.ts";
import styles from "./BuildSteps.module.css";

export type BuildStepsProps = {
  /** The steps of the last build, in the order they ran. */
  steps: readonly BuildStep[];
  /** Tüfteln shows each step's identifier beside its words. */
  depth: Depth;
};

const COLUMNS: readonly TableColumn[] = [
  { key: "name", label: "Schritt" },
  { key: "result", label: "Ergebnis" },
  { key: "time", label: "Dauer", width: 96, align: "end" },
];

function rowsOf(steps: readonly BuildStep[], depth: Depth): TableRow[] {
  return steps.map((step) => {
    const { name, result } = stepLabel(step, depth);
    return {
      key: step.id,
      attrs: { "data-step": step.id, "data-status": step.status },
      cells: {
        name: (
          <span className={styles.name}>
            {name}
            {depth === "tuefteln" && <span className={styles.id}>{step.id}</span>}
          </span>
        ),
        result,
        time: formatSeconds(step.ms),
      },
    };
  });
}

/**
 * The steps of the last build (spec 4.2 „Anzeige“): the ones that ran as a table (Schritt, Ergebnis, Dauer), below them the folded
 * section „Übersprungen (N)“ with a row per skipped step. Steps that only watch a folder are not shown.
 */
export function BuildSteps({ steps, depth }: BuildStepsProps) {
  const [open, setOpen] = useState(false);
  const shown = steps.filter((s) => !s.id.startsWith(WATCH_PREFIX));
  const ran = shown.filter((s) => s.status !== "skipped");
  const skipped = shown.filter((s) => s.status === "skipped");
  return (
    <div className={styles.steps} data-build-steps="">
      {ran.length > 0 ? (
        <div className={styles.table}>
          <Table columns={COLUMNS} rows={rowsOf(ran, depth)} ariaLabel="Schritte des letzten Builds" />
        </div>
      ) : (
        <p className={styles.text}>Kein Schritt lief: alles war aktuell.</p>
      )}
      {skipped.length > 0 && (
        <div className={styles.table}>
          <Section title={`Übersprungen (${skipped.length})`} open={open} onToggle={() => setOpen((o) => !o)}>
            <Table columns={COLUMNS} rows={rowsOf(skipped, depth)} ariaLabel="Übersprungene Schritte" />
          </Section>
        </div>
      )}
    </div>
  );
}
