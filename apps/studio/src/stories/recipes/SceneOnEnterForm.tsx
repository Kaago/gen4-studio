import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { HeaderSummary, Step } from "../../api";
import { Button } from "../../ui/primitives";
import { headerLabel } from "../../workbench/headers";
import { HeaderPicker } from "../../workbench/inspector/HeaderPicker";
import { SaysField } from "./recipeFields";
import { StepField } from "./StepField";
import type { StoryRecipeForm } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type SceneOnEnterFormProps = {
  form: Extract<StoryRecipeForm, { recipe: "scene_on_enter" }>;
  onChange: (form: Extract<StoryRecipeForm, { recipe: "scene_on_enter" }>) => void;
  steps: readonly Step[] | null;
  headers: readonly HeaderSummary[];
  disabled?: boolean;
};

/** „Szene beim Betreten“: the place, the step at which entering it shows the text, and the text. */
export function SceneOnEnterForm({ form, onChange, steps, headers, disabled }: SceneOnEnterFormProps) {
  const { t } = useTranslation("stories");
  const [picking, setPicking] = useState(false);
  const place = headers.find((h) => h.id === form.header);
  return (
    <>
      <div className={styles.field}>
        <span className={styles.label}>{t("sceneOnEnterForm.place")}</span>
        <div className={styles.line}>
          <span data-scene-place="">{place === undefined ? t("sceneOnEnterForm.noPlaceChosen") : headerLabel(place)}</span>
          <Button kind="quiet" size="s" aria-label={t("sceneOnEnterForm.choosePlaceLabel")} disabled={disabled} onClick={() => setPicking(true)}>
            {t("sceneOnEnterForm.choose")}
          </Button>
        </div>
      </div>
      <HeaderPicker open={picking} onOpenChange={setPicking} headers={headers} title={t("sceneOnEnterForm.choosePlaceTitle")} onPick={(header) => onChange({ ...form, header })} />
      <StepField label={t("stepField.step")} steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <SaysField value={form.text} onChange={(text) => onChange({ ...form, text })} disabled={disabled} />
    </>
  );
}
