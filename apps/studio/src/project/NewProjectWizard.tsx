import { useContext, useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../api";
import { t } from "../i18n";
import type { Created, CreatePhase, CreateProgress, DestCheck, RomInfo, Template } from "../api";
import { openPath } from "../platform/dialogs";
import { Badge, Button, CheckList, Checkbox, DSScreen, Icon, Notice, Switch, TextField } from "../ui/primitives";
import type { CheckRow } from "../ui/primitives";
import { initDoneText, isNotInstalled, templatesOnlyText } from "../git/gitWords";
import { DoneResult } from "../workbench/wizards/WizardResult";
import { WizardFrame } from "../workbench/wizards/WizardFrame";
import type { WizardStep } from "../workbench/wizards/WizardFrame";
import parts from "../workbench/wizards/WizardStep.module.css";
import { destOf, gameName, languageName, nameProblem, suggestName } from "./projectName";
import { StartServiceContext } from "./startService";
import { destProblem, megabytes, startMessage } from "./startMessages";
import styles from "./NewProjectWizard.module.css";

/** What the wizard has chosen so far, for the view above it. */
export type WizardChoice = { rom: RomInfo | null; template: Template; name: string; dest: string | null; textProject: boolean; git: boolean };

type Props = {
  /**
   * A ROM path to start with (a dropped file): read at once; a released ROM moves on to the step "Vorlage", an untested one stays on
   * "ROM" so its warning is read.
   */
  initialRom?: string;
  /** A note on the step "ROM", e.g. why a dropped file brought no path. */
  initialNotice?: string;
  onCancel: () => void;
  /** „Öffnen“ on the result. */
  onOpen: (created: Created) => void;
  onChoice?: (choice: WizardChoice) => void;
};

/** How long the destination check waits for the typing to stop. */
const CHECK_DELAY_MS = 200;

/** The catalog key of each template (`project:templates`). */
const TEMPLATE_KEY = { original: "original", data_only: "dataOnly", empty_region: "emptyRegion" } as const satisfies Record<Template, string>;

/** How the summary and the start's view name a template, in the language shown. */
export const templateName = (template: Template): string => t(`project:templates.${TEMPLATE_KEY[template]}`);

/** The template cards, in their order; title and text are read at render (`project:wizard.cards`). */
const CARDS: readonly Template[] = ["original", "empty_region", "data_only"];

/** The kernel's own words for a game code that is no Gen-4 code (`g4_kernel::start::NOT_GEN4`): compared, never shown. */
const KERNEL_NOT_GEN4 = "Kein Gen-4-Spiel erkannt";

type PhaseState = "open" | "running" | "done" | "failed";
type PhaseRow = { id: CreatePhase; state: PhaseState; done: number; total: number };

/** The phases of a creation in their order, by the key of their label (`project:wizard.phases`). */
const PHASES = [
  ["read", "read"],
  ["extract", "extract"],
  ["template", "template"],
  ["check", "check"],
  ["text_project", "textProject"],
  ["git", "git"],
] as const satisfies readonly (readonly [CreatePhase, string])[];

/** The label of a phase in the language shown. */
const phaseLabel = (id: CreatePhase): string => {
  const key = PHASES.find(([phase]) => phase === id)?.[1];
  return key === undefined ? id : t(`project:wizard.phases.${key}`);
};

/** The phases of a creation; setting it up as text project and for Git only when asked for. */
const freshPhases = (textProject: boolean, git: boolean): PhaseRow[] =>
  PHASES.filter(([id]) => (textProject || id !== "text_project") && (git || id !== "git")).map(([id]) => ({ id, state: "open", done: 0, total: 0 }));

/**
 * One progress event: the phases before it are done; the phase itself runs, or is done when the event says so (the dev
 * bridge reports each phase done after the work, Tauri reports each as it starts).
 */
function advance(rows: PhaseRow[], p: CreateProgress): PhaseRow[] {
  const at = rows.findIndex((r) => r.id === p.phase);
  if (at < 0) return rows;
  return rows.map((r, i): PhaseRow => {
    if (i < at) return { ...r, state: "done" };
    if (i > at) return r;
    const finished = p.total > 0 && p.done >= p.total;
    return { ...r, state: finished ? "done" : "running", done: p.done, total: p.total };
  });
}

/** The work failed: the phase it was in (the first one not done) failed. */
function fail(rows: PhaseRow[]): PhaseRow[] {
  const at = rows.findIndex((r) => r.state !== "done");
  return rows.map((r, i) => (i === at ? { ...r, state: "failed" } : r));
}

const CHECK_STATE: Record<PhaseState, CheckRow["state"]> = { open: "info", running: "info", done: "ok", failed: "error" };

function phaseNote(r: PhaseRow): string {
  switch (r.state) {
    case "open":
      return t("project:wizard.phaseNote.open");
    case "running":
      return r.total > 1 ? t("project:wizard.phaseNote.runningCount", { done: r.done, total: r.total }) : t("project:wizard.phaseNote.running");
    case "done":
      return t("project:wizard.phaseNote.done");
    case "failed":
      return t("project:wizard.phaseNote.failed");
  }
}

const releasable = (info: RomInfo | null) => info?.support.kind === "verified" || info?.support.kind === "untested";

/** The plaque of a ROM's release. */
function SupportBadge({ support }: { support: RomInfo["support"] }) {
  const { t } = useTranslation("project");
  switch (support.kind) {
    case "verified":
      return <Badge kind="success">{t("wizard.support.verified")}</Badge>;
    case "untested":
      return <Badge kind="warning">{t("wizard.support.untested")}</Badge>;
    case "locked":
      return <Badge kind="locked">{t("wizard.support.locked")}</Badge>;
    case "unknown":
      return <Badge kind="error">{t("wizard.support.unknown")}</Badge>;
  }
}

/** What the header says: the title on a DS screen, like the label of a cartridge, the facts beside it. */
function RomCard({ info }: { info: RomInfo }) {
  const { t } = useTranslation("project");
  const { support } = info;
  // The badge says „Kein Gen-4-Spiel erkannt“ already: a reason that only repeats it is not shown again.
  const repeatsBadge = support.kind === "unknown" && (support.reason === KERNEL_NOT_GEN4 || support.reason === t("wizard.support.unknown"));
  const why = support.kind === "untested" ? t("wizard.untested") : support.kind === "locked" || (support.kind === "unknown" && !repeatsBadge) ? support.reason : null;
  return (
    <>
      <div className={styles.rom}>
        <div className={styles.screen}>
          <DSScreen variant="textbox" pages={[{ lines: [[{ kind: "text", text: info.title }]], prompt: false, overflow: false }]} page={0} scale={1} />
        </div>
        <dl className={styles.facts}>
          <div>
            <dt>{t("wizard.facts.game")}</dt>
            <dd>{gameName(info)}</dd>
          </div>
          <div>
            <dt>{t("wizard.facts.language")}</dt>
            <dd>{languageName(info.language)}</dd>
          </div>
          <div>
            <dt>{t("wizard.facts.gameCode")}</dt>
            <dd className={styles.code}>{info.game_code}</dd>
          </div>
          <div>
            <dt>{t("wizard.facts.support")}</dt>
            <dd>
              <SupportBadge support={support} />
            </dd>
          </div>
          <div>
            <dt>{t("wizard.facts.base")}</dt>
            <dd>{info.known_base ? t("wizard.facts.knownBase", { base: info.known_base }) : t("wizard.facts.unknownBase")}</dd>
          </div>
        </dl>
      </div>
      {why && <p className={support.kind === "untested" ? parts.warn : parts.text}>{why}</p>}
    </>
  );
}

/** The template cards: a radio group, the arrows move the choice and wrap. */
function TemplateCards({ value, untested, onChange }: { value: Template; untested: boolean; onChange: (t: Template) => void }) {
  const { t } = useTranslation("project");
  const headingId = useId();
  const group = useRef<HTMLDivElement>(null);
  function onKey(e: KeyboardEvent<HTMLDivElement>) {
    const step = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    e.preventDefault();
    const at = CARDS.indexOf(value);
    const next = CARDS[(at + step + CARDS.length) % CARDS.length];
    onChange(next);
    group.current?.querySelector<HTMLElement>(`[data-template="${next}"]`)?.focus();
  }
  return (
    <>
      <h3 id={headingId} className={parts.heading}>
        {t("wizard.templateHeading")}
      </h3>
      <div ref={group} role="radiogroup" aria-labelledby={headingId} className={styles.cards} onKeyDown={onKey}>
        {CARDS.map((id) => {
          const chosen = id === value;
          const card = { title: t(`wizard.cards.${TEMPLATE_KEY[id]}.title`), text: t(`wizard.cards.${TEMPLATE_KEY[id]}.text`) };
          return (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={chosen}
              tabIndex={chosen ? 0 : -1}
              data-template={id}
              className={styles.card}
              onClick={() => onChange(id)}
            >
              <span className={styles.cardHead}>
                <span className={styles.mark} aria-hidden="true">
                  <Icon name={chosen ? "success" : "unsaved"} weight={chosen ? "fill" : "regular"} />
                </span>
                <span className={styles.cardTitle}>{card.title}</span>
              </span>
              <span className={styles.cardText}>{card.text}</span>
              {untested && (
                <span className={styles.badges}>
                  <Badge kind="warning">{t("wizard.support.untested")}</Badge>
                </span>
              )}
            </button>
          );
        })}
      </div>
    </>
  );
}

type Checked = { dest: string; result: DestCheck | null; problem: string | null };

/** "Neues Spiel aus ROM": a ROM, a template, a name and a folder, then the ROM is unpacked into a new project. */
export function NewProjectWizard({ initialRom, initialNotice, onCancel, onOpen, onChoice }: Props) {
  const { t } = useTranslation("project");
  const { inspectRom, defaultProjectParent, checkProjectDest, createProject } = useContext(StartServiceContext);
  const [step, setStep] = useState(0);
  const [romPath, setRomPath] = useState(initialRom ?? "");
  const [rom, setRom] = useState<RomInfo | null>(null);
  const [romError, setRomError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [template, setTemplate] = useState<Template>("original");
  // Off by default: DSPRE users decide themselves; it costs 10 to 120 s more.
  const [textProject, setTextProject] = useState(false);
  // Off by default as well (plan 4.4 „Orte von Git einrichten“): a local repository with a first commit, about 10 s more.
  const [git, setGit] = useState(false);
  const [name, setName] = useState("");
  // A name the user typed is kept when another ROM is read; a proposed one follows the ROM.
  const ownName = useRef(false);
  const [parent, setParent] = useState("");
  const [parentError, setParentError] = useState<string | null>(null);
  const [dspre, setDspre] = useState(false);
  const [checked, setChecked] = useState<Checked | null>(null);
  // Counts up when the destination must be asked again although its path did not change (after a failed create).
  const [recheck, setRecheck] = useState(0);
  const [busy, setBusy] = useState(false);
  const [phases, setPhases] = useState(() => freshPhases(false, false));
  const [error, setError] = useState<string | null>(null);
  const [created, setCreated] = useState<Created | null>(null);

  // Only the answer for the path read last counts.
  const reads = useRef(0);
  async function readRom(path: string, jump: boolean) {
    const wanted = path.trim();
    const id = ++reads.current;
    setRomPath(wanted);
    setRom(null);
    setRomError(null);
    if (wanted === "") {
      setReading(false);
      return;
    }
    setReading(true);
    try {
      const info = await inspectRom(wanted);
      if (id !== reads.current) return;
      setRom(info);
      if (!ownName.current) setName(suggestName(info));
      if (jump && info.support.kind === "verified") setStep(1);
    } catch (e) {
      if (id === reads.current) setRomError(errorText(e));
    } finally {
      if (id === reads.current) setReading(false);
    }
  }

  useEffect(() => {
    let live = true;
    defaultProjectParent().then(
      (p) => {
        if (!live) return;
        setParent(p.parent);
        setDspre(p.dspre_suffix);
      },
      (e) => live && setParentError(errorText(e)),
    );
    if (initialRom) void readRom(initialRom, true);
    return () => {
      live = false;
    };
  }, []);

  const problem = nameProblem(name);
  const dest = parent.trim() !== "" && problem === null ? destOf(parent.trim(), name, dspre) : null;
  const size = rom?.size ?? null;

  // The destination is asked once the typing stopped; an answer for an older path is dropped.
  useEffect(() => {
    setChecked(null);
    if (dest === null || size === null) return;
    let live = true;
    const timer = setTimeout(() => {
      checkProjectDest(dest, size).then(
        (result) => live && setChecked({ dest, result, problem: destProblem(result) }),
        (e) => live && setChecked({ dest, result: null, problem: errorText(e) }),
      );
    }, CHECK_DELAY_MS);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [dest, size, recheck]);

  const current = checked !== null && checked.dest === dest ? checked : null;
  const destOk = current !== null && current.problem === null;

  const choice = useRef(onChoice);
  choice.current = onChoice;
  useEffect(() => {
    choice.current?.({ rom, template, name, dest, textProject, git });
  }, [rom, template, name, dest, textProject, git]);

  async function pickRom() {
    const path = await openPath({ directory: false, title: t("wizard.pickRom"), filters: [{ name: "DS-ROM", extensions: ["nds"] }] });
    if (path) void readRom(path, false);
  }

  async function pickFolder() {
    const path = await openPath({ directory: true, title: t("wizard.pickFolder") });
    if (!path) return;
    setParentError(null);
    setParent(path);
  }

  async function create() {
    if (!rom || dest === null) return;
    setBusy(true);
    setError(null);
    setPhases(freshPhases(textProject, git));
    try {
      const made = await createProject(romPath, dest, template, textProject, git, (p) => setPhases((rows) => advance(rows, p)));
      setPhases((rows) => rows.map((r) => ({ ...r, state: "done" })));
      setCreated(made);
    } catch (e) {
      setPhases(fail);
      setError(startMessage(errorText(e)));
      // The failed work may have left the destination otherwise than the last check saw it.
      setRecheck((n) => n + 1);
    } finally {
      setBusy(false);
    }
  }

  const game = rom ? `${gameName(rom)} (${languageName(rom.language)})` : "";
  const ready = releasable(rom) && dest !== null && destOk;
  // The empty region needs rotom's sources: creating the project adopts the scripts first.
  const adopts = template === "empty_region" ? t("wizard.summary.adopts") : "";
  const asText = `${textProject ? t("wizard.summary.asTextProject") : ""}${git ? t("wizard.summary.withGit") : ""}`;
  const sentence = ready ? t("wizard.summary.sentence", { name: name.trim(), game, template: templateName(template), options: asText, dest, adopts }) : undefined;

  // Something already lies at the project's own path: a question of the name. Anything else (space, a refusal) is the folder's.
  const taken = current?.result?.state === "not_empty" || current?.result?.state === "not_a_directory";
  const nameError = problem ?? (taken ? (current?.problem ?? undefined) : undefined);
  let folderError: string | undefined;
  if (parentError !== null) folderError = parentError;
  else if (parent.trim() === "") folderError = t("wizard.noFolder");
  else if (current?.problem && !taken) folderError = current.problem;

  let destNote: string;
  if (dest === null) destNote = "";
  else if (current === null) destNote = t("wizard.checkingFolder");
  else if (current.result && current.problem === null) {
    const { free_bytes: freeBytes, needed_bytes: needed } = current.result;
    destNote = freeBytes === null ? t("wizard.needed", { needed: megabytes(needed) }) : t("wizard.space", { free: megabytes(freeBytes), needed: megabytes(needed) });
  } else destNote = "";

  const steps: WizardStep[] = [
    {
      id: "rom",
      title: t("wizard.steps.rom"),
      canNext: !reading && releasable(rom),
      content: (
        <>
          <section className={parts.column}>
            {initialNotice && <Notice kind="info">{initialNotice}</Notice>}
            <div className={styles.pathField}>
              <TextField label={t("wizard.romFile")} showLabel value={romPath} onCommit={(path) => void readRom(path, false)} placeholder={t("wizard.romPlaceholder")} error={romError ?? undefined} />
            </div>
            <Button kind="secondary" icon="open" onClick={() => void pickRom()}>
              {t("wizard.chooseFile")}
            </Button>
            {reading && <p className={parts.note}>{t("wizard.readingRom")}</p>}
            <p className={parts.note}>{t("wizard.ownRom")}</p>
          </section>
          {rom && (
            <section className={`${parts.column} ${parts.wide}`}>
              <RomCard info={rom} />
            </section>
          )}
        </>
      ),
    },
    {
      id: "template",
      title: t("wizard.steps.template"),
      canNext: true,
      content: (
        <section className={`${parts.column} ${parts.wide}`}>
          <TemplateCards value={template} untested={rom?.support.kind === "untested"} onChange={setTemplate} />
          <div className={parts.group}>
            <Checkbox
              label={t("wizard.textProject")}
              checked={textProject}
              onChange={(on) => {
                setTextProject(on);
                // The phase list follows the box (the work has not started on this step).
                setPhases(freshPhases(on, git));
              }}
            />
            <p className={parts.note}>{t("wizard.textProjectHint")}</p>
          </div>
          <div className={parts.group}>
            <Checkbox
              label={t("wizard.git")}
              checked={git}
              onChange={(on) => {
                setGit(on);
                setPhases(freshPhases(textProject, on));
              }}
            />
            <p className={parts.note}>{t("wizard.gitHint")}</p>
          </div>
        </section>
      ),
    },
    {
      id: "dest",
      title: t("wizard.steps.dest"),
      canNext: problem === null && destOk,
      content: (
        <>
          <section className={parts.column}>
            <div className={styles.pathField}>
              <TextField
                label={t("wizard.name")}
                showLabel
                value={name}
                onCommit={(typed) => {
                  ownName.current = true;
                  setName(typed);
                }}
                error={nameError}
              />
            </div>
            <div className={styles.pathField}>
              <TextField label={t("wizard.folder")} showLabel value={parent} onCommit={(typed) => {
                  setParentError(null);
                  setParent(typed.trim());
                }} error={folderError} />
            </div>
            <Button kind="secondary" icon="open" onClick={() => void pickFolder()}>
              {t("wizard.chooseFolder")}
            </Button>
          </section>
          <section className={`${parts.column} ${parts.wide}`}>
            <Switch label={t("wizard.dspreSuffix")} checked={dspre} onChange={setDspre} />
            <div className={parts.group}>
              <span className={parts.label}>{t("wizard.newProject")}</span>
              <p className={styles.path}>{dest ?? "–"}</p>
            </div>
            {destNote && <p className={parts.note}>{destNote}</p>}
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: t("wizard.steps.summary"),
      canNext: ready,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("wizard.summaryHeading")}</h3>
            <dl className={styles.facts}>
              <div>
                <dt>{t("wizard.facts.rom")}</dt>
                <dd className={styles.code}>{romPath}</dd>
              </div>
              <div>
                <dt>{t("wizard.facts.game")}</dt>
                <dd>{game}</dd>
              </div>
              <div>
                <dt>{t("wizard.facts.template")}</dt>
                <dd>{`${templateName(template)}${asText}`}</dd>
              </div>
              <div>
                <dt>{t("wizard.facts.folder")}</dt>
                <dd className={styles.code}>{dest ?? "–"}</dd>
              </div>
            </dl>
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("wizard.progress")}</h3>
            <div className={styles.progress}>
              <CheckList
                ariaLabel={t("wizard.progress")}
                rows={phases.map((r) => ({
                  id: r.id,
                  state: CHECK_STATE[r.state],
                  text: phaseLabel(r.id),
                  trailing: <span className={styles.phaseNote}>{phaseNote(r)}</span>,
                  dataAttrs: { "data-phase": r.state },
                }))}
              />
            </div>
          </section>
        </>
      ),
    },
  ];

  // „Git einrichten“ when it was asked for: the first commit in the success text, otherwise why there is none (only the templates without
  // `git`, Git's reason for a failed commit) as a warning; the project is there either way.
  const gitReport = created?.git ?? null;
  const gitWarning = gitReport === null || gitReport.committed || gitReport.reason === null ? null : isNotInstalled(gitReport.reason) ? templatesOnlyText() : gitReport.reason;
  const result = created ? (
    <div className={styles.result}>
      <DoneResult
        title={t("wizard.done.title")}
        lines={[created.root, t("wizard.done.sha1", { sha1: created.sha1.slice(0, 8) }), ...(gitReport?.committed ? [initDoneText(gitReport)] : [])]}
        next={[created.template === "data_only" ? t("wizard.done.nextDataOnly") : t("wizard.done.next")]}
      />
      {gitWarning !== null && <Notice kind="warning">{gitWarning}</Notice>}
    </div>
  ) : null;

  return (
    <WizardFrame
      title={t("wizard.title")}
      icon="open"
      steps={steps}
      step={step}
      onStep={(i) => {
        setError(null);
        setStep(i);
      }}
      busy={busy}
      error={error}
      summary={step === steps.length - 1 ? sentence : undefined}
      result={result}
      submitLabel={t("wizard.submit")}
      onSubmit={() => void create()}
      onCancel={onCancel}
      onShow={created ? () => onOpen(created) : undefined}
      showLabel={t("wizard.open")}
    />
  );
}
