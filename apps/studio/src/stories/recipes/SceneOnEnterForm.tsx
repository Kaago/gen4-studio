import { useState } from "react";
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
  const [picking, setPicking] = useState(false);
  const place = headers.find((h) => h.id === form.header);
  return (
    <>
      <div className={styles.field}>
        <span className={styles.label}>Ort</span>
        <div className={styles.line}>
          <span data-scene-place="">{place === undefined ? "Keiner gewählt" : headerLabel(place)}</span>
          <Button kind="quiet" size="s" aria-label="Ort wählen…" disabled={disabled} onClick={() => setPicking(true)}>
            Wählen…
          </Button>
        </div>
      </div>
      <HeaderPicker open={picking} onOpenChange={setPicking} headers={headers} title="Ort wählen" onPick={(header) => onChange({ ...form, header })} />
      <StepField label="Schritt" steps={steps} value={form.step} onChange={(step) => onChange({ ...form, step })} disabled={disabled} />
      <SaysField value={form.text} onChange={(text) => onChange({ ...form, text })} disabled={disabled} />
    </>
  );
}
