import { useEffect, useRef, useState } from "react";
import { applyEmptyRegion, errorText, nameTables, planEmptyRegion } from "../api";
import type { AdoptProgressDto, AdoptReportDto, DraftDto, RegionPlan, RegionReport, ScriptsStatusDto } from "../api";
import { t } from "../i18n";
import type { Depth } from "../ui/frame/shellState";
import { Notice } from "../ui/primitives";
import { AdoptProgress } from "../workbench/script/AdoptBox";
import { nextStepsFor } from "../workbench/recipes";
import { id4, termLabel, termPlural } from "../workbench/terms";
import { DraftNotes } from "../workbench/wizards/DraftNotes";
import { useDraftPlan } from "../workbench/wizards/useDraftPlan";
import { DoneResult } from "../workbench/wizards/WizardResult";
import { WizardFrame } from "../workbench/wizards/WizardFrame";
import type { WizardStep } from "../workbench/wizards/WizardFrame";
import parts from "../workbench/wizards/WizardStep.module.css";
import styles from "./ClearWorldWizard.module.css";

export interface ClearWorldWizardProps {
  /** The view depth: only Tüfteln shows the kernel's sentences of the draft (they speak of headers and numbers). */
  depth: Depth;
  /** The project's script status (null: not read yet): the recipe writes rotom sources, so the scripts must be adopted. */
  scripts: ScriptsStatusDto | null;
  /** „Scripts übernehmen“: decompiles the project, calling `onProgress` as files are done; the app's status says adopted afterwards. */
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
  /** A next step's button on the result („Gebiet anlegen“): the app shows the town and opens that recipe's wizard. */
  onStep?: (id: string, report: RegionReport) => void;
  /** The draft of the recipe (the border cells of the world map) while it can run (null: nothing planned): the scene and the world map draw it. */
  onDraft: (draft: DraftDto | null) => void;
}

type PlanState = { kind: "loading" } | { kind: "ready"; plan: RegionPlan } | { kind: "failed"; message: string };

/** The empty region takes no input. */
const EMPTY_REGION_INPUT = {};
/** The kernel's refusals of `applyEmptyRegion`: the key of the wizard's words for each (`project:clearWorld`); anything else is shown as it comes. */
const REFUSALS = { "the world is already empty": "alreadyEmpty", "scripts are not adopted": "adoptFirst", "save first": "saveFirst" } as const;
const refusalOf = (text: string): string | undefined => {
  const key = (REFUSALS as Readonly<Record<string, (typeof REFUSALS)[keyof typeof REFUSALS]>>)[text];
  return key === undefined ? undefined : t(`project:clearWorld.${key}`);
};

/** The town as the game names it, else its header. */
const townOf = (plan: RegionPlan) => plan.town_name || `${termLabel("header")} ${id4(plan.start_header)}`;

/** What the app says once it showed the town: "Welt geleert, Zweiblattdorf bleibt." */
export const clearedAnnouncement = (report: RegionReport) => t("project:clearWorld.cleared", { town: townOf(report.plan) });

/** What stays: the town with its rooms (every kept header but the town itself). */
function townLine(plan: RegionPlan): string {
  const rooms = plan.kept.length - 1;
  if (rooms <= 0) return t("project:clearWorld.townStays", { town: townOf(plan) });
  return t("project:clearWorld.townStaysWithRooms", { town: townOf(plan), count: rooms });
}

const emptiedLine = (n: number): string => t("project:clearWorld.emptied", { count: n });

const borderLine = (n: number): string => t("project:clearWorld.border", { count: n });

const starterLine = (plan: RegionPlan, species: string) => t("project:clearWorld.starter", { species, level: plan.starter_level });

/** "Welt leeren": the empty-region recipe on the open project. Shows what stays and what goes, clears what is in the way, then applies it. */
export function ClearWorldWizard({ depth, scripts, onAdopt, unsaved, onSave, onApplied, onClose, onShow, onPlay, onStep, onDraft }: ClearWorldWizardProps) {
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
  // A run failed for a reason the kernel does not refuse up front: files may have been written, so the plan is stale and
  // "Welt leeren" stays off (a second run would write another border map, level script and start script); reopening plans again.
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
      const known = refusalOf(text);
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
  const speciesName = plan ? species?.[plan.starter_species]?.trim() || t("project:clearWorld.species", { number: plan.starter_species }) : "";
  const lines = plan ? [townLine(plan), emptiedLine(plan.emptied.length), borderLine(plan.border_cells), starterLine(plan, speciesName)] : [];
  const ready = plan !== null && !empty && !touched && scripts?.adopted === true && unsaved === 0 && adopting === null && !saving;
  // The recipe's draft while „Welt leeren“ can run; gone once it ran.
  const preview = useDraftPlan("empty_region", ready && report === null ? EMPTY_REGION_INPUT : null, onDraft);

  let what;
  if (planState.kind === "loading") {
    what = (
      <section className={parts.column}>
        <p className={parts.note}>{t("project:clearWorld.reading")}</p>
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
          <h3 className={parts.heading}>{t("project:clearWorld.stays")}</h3>
          <p className={parts.text}>{lines[0]}</p>
          <p className={parts.text}>{lines[3]}</p>
        </section>
        {!empty && (
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("project:clearWorld.goes")}</h3>
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
        {t("project:clearWorld.alreadyEmpty")}
      </Notice>,
    );
  } else if (plan) {
    if (scripts === null) needs.push(<p key="status" className={parts.note}>{t("project:clearWorld.checkingScripts")}</p>);
    else if (!scripts.adopted || adopting !== null) {
      needs.push(
        <Notice key="adopt" kind="warning" actions={adopting === null ? [{ label: t("project:clearWorld.adopt"), onClick: () => void adopt() }] : undefined}>
          <div className={styles.adopt}>
            <p>{t("project:clearWorld.adoptFirst")}</p>
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
    if (binary > 0) needs.push(<p key="binary" className={parts.note}>{t("project:clearWorld.binary", { count: binary, term: binary === 1 ? termLabel("scriptFile") : termPlural("scriptFile") })}</p>);
    if (unsaved > 0) {
      needs.push(
        <Notice key="save" kind="warning" actions={[{ label: t("project:clearWorld.save"), onClick: () => void save() }]}>
          {t("project:clearWorld.saveFirst")}
        </Notice>,
      );
    }
  }

  const steps: WizardStep[] = [
    {
      id: "plan",
      title: t("project:clearWorld.stepPlan"),
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
      title: t("project:clearWorld.stepSummary"),
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
          <DraftNotes draft={preview.draft} error={preview.error} depth={depth} />
        </section>
      ),
    },
  ];

  const summary = plan && !empty ? t("project:clearWorld.summary", { town: townOf(plan) }) : undefined;

  let result = null;
  let showIt: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (report) {
    const [file, n] = report.start_script;
    result = (
      <DoneResult
        title={t("project:clearWorld.doneTitle")}
        lines={[t("project:clearWorld.doneFiles", { border: id4(report.border_map), level: id4(report.level_script), file: id4(file), n })]}
        next={[t("project:clearWorld.doneNext")]}
        // The step the kernel names after the empty region; the wizard still calls the old command, which answers none.
        steps={nextStepsFor("empty_region")}
        onStep={onStep && ((id) => onStep(id, report))}
      />
    );
    showIt = () => void show(report);
    if (onPlay) play = () => void showAndPlay(report);
  }

  return (
    <WizardFrame
      title={t("project:clearWorld.title")}
      icon="world"
      steps={steps}
      step={step}
      onStep={(i) => {
        // The failure that blocks the wizard stays in view.
        if (!touched) setRefusal(null);
        setStep(i);
      }}
      busy={busy || saving || adopting !== null}
      error={refusal}
      summary={summary}
      irreversible
      result={result}
      submitLabel={t("project:clearWorld.title")}
      onSubmit={() => void apply()}
      // After a run the world was read again already; a failed run that may have written files has it read.
      onCancel={() => onClose(report === null && touched)}
      onShow={showIt}
      onPlay={play}
    />
  );
}
