import { useEffect, useRef, useState } from "react";
import { applyEmptyRegion, errorText, nameTables, planEmptyRegion } from "../api";
import type { AdoptProgressDto, AdoptReportDto, RegionPlan, RegionReport, ScriptsStatusDto } from "../api";
import { Notice } from "../ui/primitives";
import { AdoptProgress } from "../workbench/script/AdoptBox";
import { TERMS, id4 } from "../workbench/terms";
import { DoneResult } from "../workbench/wizards/WizardResult";
import { WizardFrame } from "../workbench/wizards/WizardFrame";
import type { WizardStep } from "../workbench/wizards/WizardFrame";
import parts from "../workbench/wizards/WizardStep.module.css";
import styles from "./ClearWorldWizard.module.css";

export interface ClearWorldWizardProps {
  /** The project's script status (null: not read yet): the recipe writes rotom sources, so the scripts must be adopted. */
  scripts: ScriptsStatusDto | null;
  /** "Scripts übernehmen": decompiles the project, calling `onProgress` as files are done; the app's status says adopted afterwards. */
  onAdopt: (onProgress: (progress: AdoptProgressDto) => void) => Promise<AdoptReportDto>;
  /** Unsaved edits of the store and of the open scripts: the recipe writes scripts straight to the disk, so they go first. */
  unsaved: number;
  /** "Speichern": saves everything (a wizard locks ⌘S). */
  onSave: () => Promise<void>;
  /** The recipe went in: the app reads the changed assets again, as after ⌘Z. */
  onApplied: (report: RegionReport) => Promise<void>;
  /** The wizard is left; `written` says the app must read the project again (a failed run may have left files). */
  onClose: (written: boolean) => void;
  /** "Zeigen": the app shows the start town in the world. */
  onShow: (report: RegionReport) => Promise<void>;
  /** "Hier spielen" on the result: after the town was shown, the app saves, builds and runs. */
  onPlay?: () => void;
}

type PlanState = { kind: "loading" } | { kind: "ready"; plan: RegionPlan } | { kind: "failed"; message: string };

const ADOPT_FIRST = "Dafür müssen die Scripts übernommen sein.";
const SAVE_FIRST = "Speichere zuerst, das Rezept schreibt Scripts direkt auf die Platte.";
const ALREADY_EMPTY = "Die Welt ist schon leer.";

/** The kernel's refusals of `applyEmptyRegion`, in the wizard's words; anything else is shown as it comes. */
const REFUSALS: Record<string, string> = {
  "the world is already empty": ALREADY_EMPTY,
  "scripts are not adopted": ADOPT_FIRST,
  "save first": SAVE_FIRST,
};

/** The town as the game names it, else its header. */
const townOf = (plan: RegionPlan) => plan.town_name || `${TERMS.header.label} ${id4(plan.start_header)}`;

/** What the app says once it showed the town: "Welt geleert, Zweiblattdorf bleibt." */
export const clearedAnnouncement = (report: RegionReport) => `Welt geleert, ${townOf(report.plan)} bleibt.`;

/** What stays: the town with its rooms (every kept header but the town itself). */
function townLine(plan: RegionPlan): string {
  const rooms = plan.kept.length - 1;
  if (rooms <= 0) return `${townOf(plan)} bleibt.`;
  return `${townOf(plan)} bleibt mit ${rooms} ${rooms === 1 ? "Raum" : "Räumen"}.`;
}

function emptiedLine(n: number): string {
  return n === 1
    ? "1 Ort wird geleert: seine Figuren, Türen, Auslöser und Level-Scripts verschwinden. Scripts und Texte bleiben liegen."
    : `${n} Orte werden geleert: ihre Figuren, Türen, Auslöser und Level-Scripts verschwinden. Scripts und Texte bleiben liegen.`;
}

function borderLine(n: number): string {
  return n === 1 ? "1 Feld der Weltkarte wird zu einer gesperrten Randkarte." : `${n} Felder der Weltkarte werden zu einer gesperrten Randkarte.`;
}

const starterLine = (plan: RegionPlan, species: string) => `Ein Start-Script gibt dir ${species} auf Level ${plan.starter_level}, den Pokédex und die Laufschuhe.`;

/** "Welt leeren": the empty-region recipe on the open project. Shows what stays and what goes, clears what is in the way, then applies it. */
export function ClearWorldWizard({ scripts, onAdopt, unsaved, onSave, onApplied, onClose, onShow, onPlay }: ClearWorldWizardProps) {
  const [planState, setPlanState] = useState<PlanState>({ kind: "loading" });
  // The species names of the project; null: not read (or not readable), the starter goes by its number.
  const [species, setSpecies] = useState<readonly string[] | null>(null);
  const [step, setStep] = useState(0);
  // A running adoption and its last progress; its failure; how many files stayed binary.
  const [adopting, setAdopting] = useState<{ progress: AdoptProgressDto | null } | null>(null);
  const [adoptError, setAdoptError] = useState<string | null>(null);
  const [binary, setBinary] = useState(0);
  const [saving, setSaving] = useState(false);
  const [busy, setBusy] = useState(false);
  // The refusal of the last "Welt leeren" (shown in place of the summary).
  const [refusal, setRefusal] = useState<string | null>(null);
  // A run failed for a reason the kernel does not refuse up front: files may have been written.
  const [touched, setTouched] = useState(false);
  const [report, setReport] = useState<RegionReport | null>(null);
  // Progress that arrives after the adoption ended must not bring the bar back.
  const adoptRunning = useRef(false);

  useEffect(() => {
    let live = true;
    planEmptyRegion().then(
      (plan) => live && setPlanState({ kind: "ready", plan }),
      (e) => live && setPlanState({ kind: "failed", message: errorText(e) }),
    );
    nameTables().then(
      (names) => live && setSpecies(names.species),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  async function adopt() {
    adoptRunning.current = true;
    setAdoptError(null);
    setAdopting({ progress: null });
    try {
      const done = await onAdopt((progress) => adoptRunning.current && setAdopting({ progress }));
      setBinary(done.failed.length);
    } catch (e) {
      setAdoptError(errorText(e));
    } finally {
      adoptRunning.current = false;
      setAdopting(null);
    }
  }

  async function save() {
    setSaving(true);
    try {
      await onSave();
    } finally {
      setSaving(false);
    }
  }

  async function apply() {
    setBusy(true);
    setRefusal(null);
    try {
      const done = await applyEmptyRegion();
      setReport(done);
      // The world behind the wizard is read again before "Zeigen" can be pressed.
      await onApplied(done);
    } catch (e) {
      const text = errorText(e);
      const known = REFUSALS[text];
      if (known === undefined) setTouched(true);
      setRefusal(known ?? text);
    } finally {
      setBusy(false);
    }
  }

  async function show(done: RegionReport) {
    setBusy(true);
    try {
      await onShow(done);
    } finally {
      setBusy(false);
    }
  }

  /** "Hier spielen": shows the town, then plays. */
  async function showAndPlay(done: RegionReport) {
    await show(done);
    onPlay?.();
  }

  const plan = planState.kind === "ready" ? planState.plan : null;
  const empty = plan?.already_empty === true;
  const speciesName = plan ? species?.[plan.starter_species]?.trim() || `Art ${plan.starter_species}` : "";
  const lines = plan ? [townLine(plan), emptiedLine(plan.emptied.length), borderLine(plan.border_cells), starterLine(plan, speciesName)] : [];
  const ready = plan !== null && !empty && scripts?.adopted === true && unsaved === 0 && adopting === null && !saving;

  let what;
  if (planState.kind === "loading") {
    what = (
      <section className={parts.column}>
        <p className={parts.note}>Lese die Welt …</p>
      </section>
    );
  } else if (planState.kind === "failed") {
    what = (
      <section className={parts.column}>
        <Notice kind="error">{planState.message}</Notice>
      </section>
    );
  } else {
    what = (
      <>
        <section className={parts.column}>
          <h3 className={parts.heading}>Bleibt</h3>
          <p className={parts.text}>{lines[0]}</p>
          <p className={parts.text}>{lines[3]}</p>
        </section>
        {!empty && (
          <section className={parts.column}>
            <h3 className={parts.heading}>Wird geleert</h3>
            <p className={parts.text}>{lines[1]}</p>
            <p className={parts.text}>{lines[2]}</p>
          </section>
        )}
      </>
    );
  }

  // What stands in the way, each with its way out. An empty world needs nothing else.
  const needs = [];
  if (empty) {
    needs.push(
      <Notice key="empty" kind="info">
        {ALREADY_EMPTY}
      </Notice>,
    );
  } else if (plan) {
    if (scripts === null) needs.push(<p key="status" className={parts.note}>Prüfe die Scripts …</p>);
    else if (!scripts.adopted || adopting !== null) {
      needs.push(
        <Notice key="adopt" kind="warning" actions={adopting === null ? [{ label: "Scripts übernehmen", onClick: () => void adopt() }] : undefined}>
          <div className={styles.adopt}>
            <p>{ADOPT_FIRST}</p>
            {adopting !== null && <AdoptProgress progress={adopting.progress} total={scripts.file_count} />}
          </div>
        </Notice>,
      );
    }
    if (adoptError !== null) {
      needs.push(
        <Notice key="adoptError" kind="error">
          {adoptError}
        </Notice>,
      );
    }
    if (binary > 0) needs.push(<p key="binary" className={parts.note}>{`${binary} ${binary === 1 ? TERMS.scriptFile.label : TERMS.scriptFile.plural} ließen sich nicht dekompilieren und bleiben binär.`}</p>);
    if (unsaved > 0) {
      needs.push(
        <Notice key="save" kind="warning" actions={[{ label: "Speichern", onClick: () => void save() }]}>
          {SAVE_FIRST}
        </Notice>,
      );
    }
  }

  const steps: WizardStep[] = [
    {
      id: "plan",
      title: "Was bleibt",
      canNext: ready,
      content: (
        <>
          {what}
          {needs.length > 0 && <section className={parts.column}>{needs}</section>}
        </>
      ),
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: ready,
      content: (
        <section className={parts.column}>
          <Notice kind="info">
            <div className={parts.lines}>
              {lines.map((line) => (
                <p key={line}>{line}</p>
              ))}
            </div>
          </Notice>
        </section>
      ),
    },
  ];

  const summary = plan && !empty ? `Leert die Welt bis auf ${townOf(plan)}. Die Welt holst du mit ⌘Z zurück; Randkarte, Level-Script und Start-Script bleiben als Dateien.` : undefined;

  let result = null;
  let showIt: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (report) {
    const [file, n] = report.start_script;
    result = (
      <DoneResult
        title="Welt geleert"
        lines={[`Randkarte ${id4(report.border_map)}, Level-Script ${id4(report.level_script)}, Start-Script ${id4(file)} #${n}`]}
        next={["Mit „Hier spielen“: Intro, zu Hause aufwachen, draußen wartet dein erstes Pokémon."]}
      />
    );
    showIt = () => void show(report);
    if (onPlay) play = () => void showAndPlay(report);
  }

  return (
    <WizardFrame
      title="Welt leeren"
      icon="world"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      busy={busy || saving || adopting !== null}
      error={refusal}
      summary={summary}
      irreversible
      result={result}
      submitLabel="Welt leeren"
      onSubmit={() => void apply()}
      // After a run the world was read again already; a failed run that may have written files has it read.
      onCancel={() => onClose(report === null && touched)}
      onShow={showIt}
      onPlay={play}
    />
  );
}
