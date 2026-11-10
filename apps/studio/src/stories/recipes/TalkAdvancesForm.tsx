import { useTranslation } from "react-i18next";
import type { HeaderSummary, Step } from "../../api";
import { FigureField } from "./FigureField";
import { SaysField } from "./recipeFields";
import { StepField } from "./StepField";
import type { StoryRecipeForm } from "./storyRecipeModel";

export type TalkAdvancesFormProps = {
  form: Extract<StoryRecipeForm, { recipe: "talk_advances" }>;
  onChange: (form: Extract<StoryRecipeForm, { recipe: "talk_advances" }>) => void;
  steps: readonly Step[] | null;
  headers: readonly HeaderSummary[];
  revision: number;
  disabled?: boolean;
  onError: (e: unknown) => void;
};

/** „Gespräch schaltet weiter“: the figure (no trainer), the step it talks from, and what it says. */
export function TalkAdvancesForm({ form, onChange, steps, headers, revision, disabled, onError }: TalkAdvancesFormProps) {
  const { t } = useTranslation("stories");
  return (
    <>
      <FigureField header={form.figure?.header ?? null} localId={form.figure?.local_id ?? null} onChange={(figure) => onChange({ ...form, figure })} headers={headers} revision={revision} lockTrainers disabled={disabled} onError={onError} />
      <StepField label={t("stepField.fromStep")} steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <SaysField value={form.text} onChange={(text) => onChange({ ...form, text })} disabled={disabled} />
    </>
  );
}
