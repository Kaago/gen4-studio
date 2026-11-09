import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { BuildStep } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Section, Table } from "../ui/primitives";
import type { TableColumn, TableRow } from "../ui/primitives";
import { WATCH_PREFIX, formatSeconds, stepLabel } from "./stepWords";
import styles from "./BuildSteps.module.css";

export type BuildStepsProps = {
  /** The steps of the last build, in the order they ran. */
  steps: readonly BuildStep[];
  /** Tüfteln shows each step's identifier beside its words. */
  depth: Depth;
  /** The build was started with „Neu bauen (ohne Cache)“: a step without state ran fully for that, not as a first build. */
  fresh?: boolean;
};

function rowsOf(steps: readonly BuildStep[], depth: Depth, fresh: boolean): TableRow[] {
  return steps.map((step) => {
    const { name, result } = stepLabel(step, depth, fresh);
    return {
      key: step.id,
      attrs: { "data-step": step.id, "data-status": step.status },
      cells: {
        name: (
          <span className={styles.name}>
            {name}
            {depthAtLeast(depth, "tuefteln") && <span className={styles.id}>{step.id}</span>}
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
export function BuildSteps({ steps, depth, fresh = false }: BuildStepsProps) {
  const { t } = useTranslation("technik");
  const [open, setOpen] = useState(false);
  const columns: readonly TableColumn[] = [
    { key: "name", label: t("buildSteps.columns.step") },
    { key: "result", label: t("buildSteps.columns.result") },
    { key: "time", label: t("buildSteps.columns.duration"), width: 96, align: "end" },
  ];
  const shown = steps.filter((s) => !s.id.startsWith(WATCH_PREFIX));
  const ran = shown.filter((s) => s.status !== "skipped");
  const skipped = shown.filter((s) => s.status === "skipped");
  return (
    <div className={styles.steps} data-build-steps="">
      {ran.length > 0 ? (
        <div className={styles.table}>
          <Table columns={columns} rows={rowsOf(ran, depth, fresh)} ariaLabel={t("buildSteps.ariaLabel")} />
        </div>
      ) : (
        <p className={styles.text}>{t("buildSteps.nothingRan")}</p>
      )}
      {skipped.length > 0 && (
        <div className={styles.table}>
          <Section title={t("buildSteps.skippedTitle", { count: skipped.length })} open={open} onToggle={() => setOpen((o) => !o)}>
            <Table columns={columns} rows={rowsOf(skipped, depth, fresh)} ariaLabel={t("buildSteps.skippedAriaLabel")} />
          </Section>
        </div>
      )}
    </div>
  );
}
