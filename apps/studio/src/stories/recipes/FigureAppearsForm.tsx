import { useTranslation } from "react-i18next";
import type { HeaderSummary, ProjectInfo, Step } from "../../api";
import { Segmented } from "../../ui/primitives";
import { FigureField } from "./FigureField";
import { PoolFlagField } from "./recipeFields";
import { StepField } from "./StepField";
import type { StoryRecipeForm } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type FigureAppearsFormProps = {
  form: Extract<StoryRecipeForm, { recipe: "figure_appears" }>;
  onChange: (form: Extract<StoryRecipeForm, { recipe: "figure_appears" }>) => void;
  steps: readonly Step[] | null;
  headers: readonly HeaderSummary[];
  project: ProjectInfo | null;
  revision: number;
  disabled?: boolean;
  onError: (e: unknown) => void;
};


/** „Figur erscheint/verschwindet ab Schritt“: the figure (a trainer too), the step, which way, and the visibility flag from the pool. */
export function FigureAppearsForm({ form, onChange, steps, headers, project, revision, disabled, onError }: FigureAppearsFormProps) {
  const { t } = useTranslation("stories");
  const ways = [
    { value: "appears", label: t("figureAppearsForm.appears") },
    { value: "disappears", label: t("figureAppearsForm.disappears") },
  ] as const;
  return (
    <>
      <FigureField header={form.figure?.header ?? null} localId={form.figure?.local_id ?? null} onChange={(figure) => onChange({ ...form, figure })} headers={headers} revision={revision} disabled={disabled} onError={onError} />
      <StepField label={t("stepField.step")} steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <div className={styles.field}>
        <span className={styles.label}>{t("figureAppearsForm.theFigure")}</span>
        <Segmented
          ariaLabel={t("figureAppearsForm.way")}
          size="s"
          options={ways.map((w) => ({ ...w, disabled }))}
          value={form.appears ? "appears" : "disappears"}
          onChange={(way) => onChange({ ...form, appears: way === "appears" })}
        />
      </div>
      <PoolFlagField label={t("figureAppearsForm.visibilityFlag")} value={form.flag} onChange={(flag) => onChange({ ...form, flag })} project={project} revision={revision} disabled={disabled} />
    </>
  );
}
