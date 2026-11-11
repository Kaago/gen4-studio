import { useTranslation } from "react-i18next";
import type { HeaderSummary, ProjectInfo, Step } from "../../api";
import { itemName } from "../../data/pokedex/pokedexModel";
import { PickField } from "../../data/pokedex/pickers";
import { useDataArea } from "../../data/useDataArea";
import { FigureField } from "./FigureField";
import { PoolFlagField, SaysField } from "./recipeFields";
import { StepField } from "./StepField";
import { rewardSays } from "./storyRecipeModel";
import type { StoryRecipeForm } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type RewardOnceFormProps = {
  form: Extract<StoryRecipeForm, { recipe: "reward_once" }>;
  onChange: (form: Extract<StoryRecipeForm, { recipe: "reward_once" }>) => void;
  steps: readonly Step[] | null;
  headers: readonly HeaderSummary[];
  project: ProjectInfo | null;
  revision: number;
  disabled?: boolean;
  onError: (e: unknown) => void;
};

/**
 * „Belohnung einmalig“: the figure (no trainer), the step it gives from, the item (by the project's names), the text (it says „Du erhältst
 * {Gegenstand}!“ in the ROM's language until one is typed) and the flag from the pool that keeps the reward given.
 */
export function RewardOnceForm({ form, onChange, steps, headers, project, revision, disabled, onError }: RewardOnceFormProps) {
  const { t } = useTranslation("stories");
  const { names } = useDataArea("pokemon");
  return (
    <>
      <FigureField header={form.figure?.header ?? null} localId={form.figure?.local_id ?? null} onChange={(figure) => onChange({ ...form, figure })} headers={headers} revision={revision} lockTrainers disabled={disabled} onError={onError} />
      <StepField label={t("stepField.fromStep")} steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <div className={styles.field}>
        <span className={styles.label}>{t("rewardOnceForm.item")}</span>
        <PickField label={t("rewardOnceForm.item")} text={form.itemName ?? t("rewardOnceForm.noItemChosen")} choice="item" disabled={disabled} onPick={(item) => onChange({ ...form, item, itemName: itemName(names.items, item) })} />
      </div>
      <SaysField value={rewardSays(form, project?.language ?? null)} onChange={(text) => onChange({ ...form, text })} disabled={disabled} />
      <PoolFlagField label={t("rewardOnceForm.rewardFlag")} value={form.flag} onChange={(flag) => onChange({ ...form, flag })} project={project} revision={revision} disabled={disabled} />
    </>
  );
}
