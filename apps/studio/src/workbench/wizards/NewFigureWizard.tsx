import { useState } from "react";
import { errorText, recipeApply } from "../../api";
import type { DraftDto, HeaderSummary, MatrixSummary, NewFigureInput, NewFigureReport, ProjectInfo, RecipeOutcome } from "../../api";
import { FlagField } from "../../flags/FlagField";
import type { Depth } from "../../ui/frame/shellState";
import { Notice, Segmented, SelectField } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { FACINGS, MOVEMENT_WORDS } from "../inspector/eventFields";
import { FigureLookField } from "../inspector/FigureLookField";
import { MessageEditor } from "../text/MessageEditor";
import { visibleLine } from "../text/textModel";
import { DraftNotes } from "./DraftNotes";
import { PlaceField, TileFields, placeTiles } from "./PlaceField";
import { TextPreview } from "./TextPreview";
import { useDraftPlan } from "./useDraftPlan";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { defaultLook, figureResult, isLateFailure } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import styles from "./NewFigureWizard.module.css";

export interface NewFigureWizardProps {
  depth: Depth;
  headers: HeaderSummary[];
  /** The overview: the bounds of the tile fields come from the place's matrix. */
  matrices: readonly MatrixSummary[];
  /** The open project: its family names the default look, the flag field reads its names. */
  project: ProjectInfo | null;
  /** Where the figure stands when the wizard opens: the selected place, the tile last chosen in the scene. */
  prefill?: { header?: number; tile?: { x: number; z: number } };
  /** The draft of the figure as planned (null: nothing planned): the scene draws it until the wizard closes or the figure stands. */
  onDraft: (draft: DraftDto | null) => void;
  /** The figure stands (one undo step): the app reads what the recipe changed again. */
  onDone: (outcome: RecipeOutcome<NewFigureReport>, input: NewFigureInput) => Promise<void>;
  /** „Zeigen“ on the result: the app shows the place of the figure. */
  onShow?: (outcome: RecipeOutcome<NewFigureReport>, input: NewFigureInput) => void;
  /** A next step's button on the result („Dialog bearbeiten“). */
  onStep?: (id: string, outcome: RecipeOutcome<NewFigureReport>, input: NewFigureInput) => void;
  /** The wizard is left; `written` says it changed the project without the app having read it again (a partial failure). */
  onCancel: (written: boolean) => void;
  onError: (e: unknown) => void;
}

/** What the wizard asks for; a tile, a place or a text not given yet is null or empty. */
export interface FigureForm {
  header: number | null;
  x: number | null;
  z: number | null;
  /** Null: the family's default look (the kernel's, `graphics_id` stays null). */
  look: number | null;
  movement: number;
  dir: number;
  text: string;
  /** 0: always visible. */
  flag: number;
}

/** The input of `new_figure` the form makes; null while the place, the tile or the text is missing. */
export function figureInput(form: FigureForm): NewFigureInput | null {
  if (form.header === null || form.x === null || form.z === null || form.text.trim() === "") return null;
  return { header: form.header, x: form.x, z: form.z, graphics_id: form.look, movement_type: form.movement, dir: form.dir, text: form.text, hidden_flag: form.flag === 0 ? null : form.flag };
}

/** The text the plan sends while the figure says nothing yet: the scene shows the figure as soon as its place and tile are known. */
export const PLAN_TEXT = "...";

/** What the draft plans: the input, or while only the text is missing the input with `PLAN_TEXT` (null without place and tile). */
export function figurePlanInput(form: FigureForm): NewFigureInput | null {
  return figureInput(form.text.trim() === "" ? { ...form, text: PLAN_TEXT } : form);
}

type Outcome = { kind: "done"; outcome: RecipeOutcome<NewFigureReport>; input: NewFigureInput } | { kind: "partial"; message: string };

const FACING_OPTIONS = FACINGS.map((f) => ({ value: String(f.value), label: f.label }));
const wordOf = (list: readonly { value: number; label: string }[], value: number, fallback: string) => list.find((w) => w.value === value)?.label ?? `${fallback} ${value}`;

/**
 * „Neue Figur“ in Erzählen: where and how the figure stands („Wo und wie“: place, tile, look, movement, facing), what it says („Dialog“:
 * the text with Shift+Enter for a new page, the DS box beside it, the flag that hides it) and the summary; „Aufstellen“ applies the
 * recipe `new_figure` (one undo step). The scene draws the planned figure while the wizard is open.
 */
export function NewFigureWizard({ depth, headers, matrices, project, prefill, onDraft, onDone, onShow, onStep, onCancel, onError }: NewFigureWizardProps) {
  const [form, setForm] = useState<FigureForm>(() => ({ header: prefill?.header ?? null, x: prefill?.tile?.x ?? null, z: prefill?.tile?.z ?? null, look: null, movement: 0, dir: 1, text: "", flag: 0 }));
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The refusal of the last „Aufstellen“ (nothing was written), shown inside the wizard.
  const [refusal, setRefusal] = useState<string | null>(null);
  const set = (change: Partial<FigureForm>) => setForm((f) => ({ ...f, ...change }));

  const place = form.header === null ? null : (headers.find((h) => h.id === form.header) ?? null);
  const input = figureInput(form);
  // Planned from the first step on: the figure stands in the scene while its place, tile and look are chosen.
  const plan = useDraftPlan("new_figure", outcome === null ? figurePlanInput(form) : null, onDraft);
  const look = form.look ?? defaultLook(project?.family, "figure");
  const fromScene = prefill?.tile !== undefined && form.x === prefill.tile.x && form.z === prefill.tile.z;
  const located = place !== null && form.x !== null && form.z !== null;

  async function apply() {
    if (input === null) return;
    setBusy(true);
    setRefusal(null);
    try {
      const result = await recipeApply<NewFigureReport>("new_figure", input);
      setOutcome({ kind: "done", outcome: result, input });
      try {
        await onDone(result, input);
      } catch (e) {
        onError(e);
      }
    } catch (e) {
      if (isLateFailure(e)) setOutcome({ kind: "partial", message: errorText(e) });
      else setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  const where = located ? `„${headerLabel(place)}“, Kachel x ${form.x}, z ${form.z}` : null;
  const summaryLines = [
    where ?? "Noch kein Ort und keine Kachel.",
    `Aussehen ${look}, ${wordOf(MOVEMENT_WORDS, form.movement, "Bewegung")}, Blick ${wordOf(FACINGS, form.dir, "Richtung")}`,
    form.text.trim() === "" ? "Sagt noch nichts." : `Sagt: „${visibleLine(form.text)}“`,
    form.flag === 0 ? "Immer zu sehen." : `Nicht zu sehen, solange Flag ${form.flag} gesetzt ist.`,
  ];

  const steps: WizardStep[] = [
    {
      id: "where",
      title: "Wo und wie",
      canNext: located,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>Wo</h3>
            <PlaceField headers={headers} value={form.header} onChange={(header) => set({ header })} depth={depth} />
            <TileFields x={form.x} z={form.z} size={placeTiles(place, matrices)} onChange={(tile) => set(tile)} />
            {fromScene && <p className={parts.note}>Kachel zuletzt in der Szene gewählt.</p>}
            {place !== null && (form.x === null || form.z === null) && <p className={parts.note}>Noch keine Kachel: x und z eintragen oder vorher in der Szene auf die Stelle zeigen.</p>}
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Wie</h3>
            <FigureLookField value={look} dir={form.dir} onCommit={(graphicsId) => set({ look: graphicsId })} />
            <div className={parts.field}>
              <SelectField label="Bewegung" showLabel value={form.movement} options={MOVEMENT_WORDS} onChange={(movement) => set({ movement })} />
            </div>
            <div className={parts.group}>
              <span className={parts.label}>Blickrichtung</span>
              <Segmented ariaLabel="Blickrichtung" size="s" options={FACING_OPTIONS} value={String(form.dir)} onChange={(dir) => set({ dir: Number(dir) })} />
            </div>
          </section>
        </>
      ),
    },
    {
      id: "dialog",
      title: "Dialog",
      canNext: input !== null,
      content: (
        <>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>Was sie sagt</h3>
            <div className={styles.text}>
              <MessageEditor label="Was sie sagt" value={form.text} onCommit={async () => true} onDraft={(text) => set({ text })} />
            </div>
            <p className={parts.note}>Enter: neue Zeile; Umschalt+Enter: neue Seite (▼).</p>
            <FlagField
              kind="flag"
              label="Sichtbarkeits-Flag"
              value={form.flag}
              onCommit={(flag) => set({ flag })}
              depth={depth}
              project={project}
              revision={0}
              field="hidden_flag"
              hint="Solange das Flag gesetzt ist, ist die Figur nicht zu sehen; „Keins“: immer zu sehen."
            />
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Vorschau</h3>
            <TextPreview text={form.text} />
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: input !== null,
      content: (
        <section className={parts.column}>
          <Notice kind="info">
            <div className={parts.lines}>
              {summaryLines.map((line) => (
                <p key={line}>{line}</p>
              ))}
            </div>
          </Notice>
          <DraftNotes draft={plan.draft} error={plan.error} depth={depth} />
        </section>
      ),
    },
  ];

  let result = null;
  let show: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const done = outcome;
    result = <DoneResult title="Figur aufgestellt" {...figureResult(done.outcome)} onStep={onStep && ((id) => onStep(id, done.outcome, done.input))} />;
    if (onShow) show = () => onShow(done.outcome, done.input);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} depth={depth} />;
  }

  return (
    <WizardFrame
      name="new_figure"
      title="Neue Figur"
      icon="figure"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel="Aufstellen"
      onSubmit={() => void apply()}
      onCancel={() => onCancel(outcome?.kind === "partial")}
      onShow={show}
      busy={busy}
      result={result}
      error={refusal}
      summary={where === null ? undefined : `Neue Figur in ${where}`}
    />
  );
}
