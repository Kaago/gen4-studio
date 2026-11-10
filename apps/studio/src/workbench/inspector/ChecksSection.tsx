import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CheckProblemDto } from "../../api";
import type { ChecksApi } from "../../checks/useChecks";
import type { Depth } from "../../ui/frame/shellState";
import { CheckList, Section } from "../../ui/primitives";
import type { InspectorModel } from "../Inspector";
import { FixDialog } from "../problems/FixDialog";
import { FindingActions, findingRow } from "../problems/ProblemsPanel";
import { isFinding } from "../problems/problems";
import type { ProblemRow } from "../problems/problems";
import styles from "./Editor.module.css";
import { useSectionOpen } from "./foldState";

/**
 * The findings of the record the inspector shows: those that lead to it (a header, an area data, a matrix cell, which also holds the
 * findings of the map it places first, an event by file, class and index), and for a building those of its map at that building.
 */
export function findingsFor(rows: readonly ProblemRow[], model: InspectorModel): CheckProblemDto[] {
  const matches = (f: CheckProblemDto): boolean => {
    const t = f.target;
    switch (model.kind) {
      case "header":
        return t?.kind === "header" && t.id === model.id;
      case "area":
        return t?.kind === "area" && t.id === model.id;
      case "matrixCell":
        return t?.kind === "matrixCell" && t.matrix === model.matrix && t.x === model.x && t.y === model.y;
      case "event":
        return t?.kind === "event" && t.eventFile === model.eventFile && t.eventKind === model.eventKind && t.index === model.index;
      case "building":
        return f.subject.kind === "map" && f.subject.index === model.map && f.key === `building:${model.index}`;
      default:
        return false;
    }
  };
  return rows.filter(isFinding).filter(matches);
}

/** The section „Prüfungen“ (spec 3b.4 §8): what the checks found at the record, with „Beheben…“ and „Ausblenden“; nothing without findings. */
export function ChecksSection({ findings, checks, depth = "bauen" }: { findings: readonly CheckProblemDto[]; checks: ChecksApi | null; depth?: Depth }) {
  const { t } = useTranslation("workbench");
  const [open, toggle] = useSectionOpen("checks", true);
  const [fixing, setFixing] = useState<CheckProblemDto | null>(null);
  if (findings.length === 0) return null;
  return (
    <>
      <Section title={t("checksSection.title")} count={findings.length} open={open} onToggle={toggle}>
        <div className={styles.pad}>
          <CheckList ariaLabel={t("checksSection.title")} rows={findings.map((f) => findingRow(f, depth, <FindingActions f={f} checks={checks} onFix={setFixing} open={false} />))} />
        </div>
      </Section>
      {fixing !== null && checks !== null && <FixDialog finding={fixing} checks={checks} depth={depth} onClose={() => setFixing(null)} />}
    </>
  );
}
