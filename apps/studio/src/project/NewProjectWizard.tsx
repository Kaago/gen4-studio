import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { checkProjectDest, createProject, defaultProjectParent, errorText, inspectRom } from "../api";
import type { Created, CreatePhase, CreateProgress, DestCheck, RomInfo, Template } from "../api";
import { openPath } from "../platform/dialogs";
import { Badge, Button, CheckList, DSScreen, Icon, Notice, Switch, TextField } from "../ui/primitives";
import type { CheckRow } from "../ui/primitives";
import { DoneResult } from "../workbench/wizards/WizardResult";
import { WizardFrame } from "../workbench/wizards/WizardFrame";
import type { WizardStep } from "../workbench/wizards/WizardFrame";
import parts from "../workbench/wizards/WizardStep.module.css";
import { destOf, gameName, languageName, nameProblem, suggestName } from "./projectName";
import { destProblem, megabytes, startMessage } from "./startMessages";
import styles from "./NewProjectWizard.module.css";

/** What the wizard has chosen so far, for the view above it. */
export type WizardChoice = { rom: RomInfo | null; template: Template; name: string; dest: string | null };

type Props = {
  /** A ROM path to start with (a dropped file): read at once; a releasable ROM moves on to the step "Vorlage". */
  initialRom?: string;
  /** A note on the step "ROM", e.g. why a dropped file brought no path. */
  initialNotice?: string;
  onCancel: () => void;
  /** "Öffnen" on the result. */
  onOpen: (created: Created) => void;
  onChoice?: (choice: WizardChoice) => void;
};

/** How long the destination check waits for the typing to stop. */
const CHECK_DELAY_MS = 200;

const TEMPLATE_NAMES: Record<Template, string> = { original: "Original", data_only: "Nur Daten" };

type Card = { id: Template | "empty_region"; title: string; text?: string; later?: string };
const CARDS: Card[] = [
  { id: "original", title: TEMPLATE_NAMES.original, text: "Das Spiel, wie es ist." },
  { id: "empty_region", title: "Leere Region mit Startstadt", later: "Kommt mit 3g.2" },
  { id: "data_only", title: TEMPLATE_NAMES.data_only, text: "Für Balancing: Werte, Lernsets, Trainer. Startet im Bereich Pokémon." },
];

const UNTESTED = "Für diese Sprache gibt es kein Testprojekt. Entpacken geht, Editoren und Vorlagen sind nicht geprüft.";
const OWN_ROM = "Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM. Die ROM bleibt unverändert.";

type PhaseState = "open" | "running" | "done" | "failed";
type PhaseRow = { id: CreatePhase; label: string; state: PhaseState; done: number; total: number };

const PHASES: { id: CreatePhase; label: string }[] = [
  { id: "read", label: "Lesen" },
  { id: "extract", label: "Entpacken" },
  { id: "template", label: "Vorlage anwenden" },
  { id: "check", label: "Prüfen" },
];

const freshPhases = (): PhaseRow[] => PHASES.map((p) => ({ ...p, state: "open", done: 0, total: 0 }));

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
      return "wartet";
    case "running":
      return r.total > 1 ? `läuft ${r.done}/${r.total}` : "läuft";
    case "done":
      return "fertig";
    case "failed":
      return "abgebrochen";
  }
}

const releasable = (info: RomInfo | null) => info?.support.kind === "verified" || info?.support.kind === "untested";

/** The plaque of a ROM's release. */
function SupportBadge({ support }: { support: RomInfo["support"] }) {
  switch (support.kind) {
    case "verified":
      return <Badge kind="success">Freigegeben</Badge>;
    case "untested":
      return <Badge kind="warning">Nicht geprüft</Badge>;
    case "locked":
      return <Badge kind="locked">Gesperrt</Badge>;
    case "unknown":
      return <Badge kind="error">Kein Gen-4-Spiel erkannt</Badge>;
  }
}

/** What the header says: the title on a DS screen, like the label of a cartridge, the facts beside it. */
function RomCard({ info }: { info: RomInfo }) {
  const { support } = info;
  const why = support.kind === "untested" ? UNTESTED : support.kind === "locked" || (support.kind === "unknown" && support.reason !== "Kein Gen-4-Spiel erkannt") ? support.reason : null;
  return (
    <>
      <div className={styles.rom}>
        <div className={styles.screen}>
          <DSScreen variant="textbox" pages={[{ lines: [[{ kind: "text", text: info.title }]], prompt: false, overflow: false }]} page={0} scale={1} />
        </div>
        <dl className={styles.facts}>
          <div>
            <dt>Spiel</dt>
            <dd>{gameName(info)}</dd>
          </div>
          <div>
            <dt>Sprache</dt>
            <dd>{languageName(info.language)}</dd>
          </div>
          <div>
            <dt>Spielcode</dt>
            <dd className={styles.code}>{info.game_code}</dd>
          </div>
          <div>
            <dt>Freigabe</dt>
            <dd>
              <SupportBadge support={support} />
            </dd>
          </div>
          <div>
            <dt>Basis</dt>
            <dd>{info.known_base ? `Bekannte Basis-ROM: ${info.known_base}` : "Unbekannte Fassung: vielleicht schon verändert."}</dd>
          </div>
        </dl>
      </div>
      {why && <p className={support.kind === "untested" ? parts.warn : parts.text}>{why}</p>}
    </>
  );
}

/** The template cards: a radio group, the arrows move the choice past the locked card. */
function TemplateCards({ value, untested, onChange }: { value: Template; untested: boolean; onChange: (t: Template) => void }) {
  const headingId = useId();
  const group = useRef<HTMLDivElement>(null);
  const choosable = CARDS.filter((c): c is Card & { id: Template } => c.later === undefined);
  function onKey(e: KeyboardEvent<HTMLDivElement>) {
    const step = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    e.preventDefault();
    const at = choosable.findIndex((c) => c.id === value);
    const next = choosable[(at + step + choosable.length) % choosable.length].id;
    onChange(next);
    group.current?.querySelector<HTMLElement>(`[data-template="${next}"]`)?.focus();
  }
  return (
    <>
      <h3 id={headingId} className={parts.heading}>
        Womit startet das Projekt?
      </h3>
      <div ref={group} role="radiogroup" aria-labelledby={headingId} className={styles.cards} onKeyDown={onKey}>
        {CARDS.map((card) => {
          const locked = card.later !== undefined;
          const chosen = card.id === value;
          return (
            <button
              key={card.id}
              type="button"
              role="radio"
              aria-checked={chosen}
              aria-disabled={locked || undefined}
              tabIndex={chosen ? 0 : -1}
              data-template={card.id}
              className={styles.card}
              onClick={() => {
                if (card.later === undefined) onChange(card.id as Template);
              }}
            >
              <span className={styles.cardHead}>
                <span className={styles.mark} aria-hidden="true">
                  <Icon name={locked ? "lock" : chosen ? "success" : "unsaved"} weight={chosen ? "fill" : "regular"} />
                </span>
                <span className={styles.cardTitle}>{card.title}</span>
              </span>
              {card.text && <span className={styles.cardText}>{card.text}</span>}
              {(locked || untested) && (
                <span className={styles.badges}>
                  {locked && <Badge kind="locked">{card.later}</Badge>}
                  {untested && <Badge kind="warning">Nicht geprüft</Badge>}
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
  const [step, setStep] = useState(0);
  const [romPath, setRomPath] = useState(initialRom ?? "");
  const [rom, setRom] = useState<RomInfo | null>(null);
  const [romError, setRomError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [template, setTemplate] = useState<Template>("original");
  const [name, setName] = useState("");
  // A name the user typed is kept when another ROM is read; a proposed one follows the ROM.
  const ownName = useRef(false);
  const [parent, setParent] = useState("");
  const [parentError, setParentError] = useState<string | null>(null);
  const [dspre, setDspre] = useState(false);
  const [checked, setChecked] = useState<Checked | null>(null);
  const [busy, setBusy] = useState(false);
  const [phases, setPhases] = useState(freshPhases);
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
      if (jump && releasable(info)) setStep(1);
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
  }, [dest, size]);

  const current = checked !== null && checked.dest === dest ? checked : null;
  const destOk = current !== null && current.problem === null;

  const choice = useRef(onChoice);
  choice.current = onChoice;
  useEffect(() => choice.current?.({ rom, template, name, dest }), [rom, template, name, dest]);

  async function pickRom() {
    const path = await openPath({ directory: false, title: "ROM-Datei wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] });
    if (path) void readRom(path, false);
  }

  async function pickFolder() {
    const path = await openPath({ directory: true, title: "Ordner wählen" });
    if (path) setParent(path);
  }

  async function create() {
    if (!rom || dest === null) return;
    setBusy(true);
    setError(null);
    setPhases(freshPhases());
    try {
      const made = await createProject(romPath, dest, template, (p) => setPhases((rows) => advance(rows, p)));
      setPhases((rows) => rows.map((r) => ({ ...r, state: "done" })));
      setCreated(made);
    } catch (e) {
      setPhases(fail);
      setError(startMessage(errorText(e)));
    } finally {
      setBusy(false);
    }
  }

  const game = rom ? `${gameName(rom)} (${languageName(rom.language)})` : "";
  const ready = releasable(rom) && dest !== null && destOk;
  const sentence = ready ? `Legt „${name.trim()}“ aus ${game} mit der Vorlage ${TEMPLATE_NAMES[template]} an, in ${dest}. Die ROM bleibt unverändert.` : undefined;

  // Something already lies at the project's own path: a question of the name. Anything else (space, a refusal) is the folder's.
  const taken = current?.result?.state === "not_empty" || current?.result?.state === "not_a_directory";
  const nameError = problem ?? (taken ? (current?.problem ?? undefined) : undefined);
  let folderError: string | undefined;
  if (parentError !== null) folderError = parentError;
  else if (parent.trim() === "") folderError = "Wähle einen Ordner für das Projekt.";
  else if (current?.problem && !taken) folderError = current.problem;

  let destNote: string;
  if (dest === null) destNote = "";
  else if (current === null) destNote = "Prüfe den Ordner …";
  else if (current.result && current.problem === null) {
    const { free_bytes: freeBytes, needed_bytes: needed } = current.result;
    destNote = freeBytes === null ? `Gebraucht etwa ${megabytes(needed)}.` : `Platz: ${megabytes(freeBytes)} frei, gebraucht etwa ${megabytes(needed)}.`;
  } else destNote = "";

  const steps: WizardStep[] = [
    {
      id: "rom",
      title: "ROM",
      canNext: !reading && releasable(rom),
      content: (
        <>
          <section className={parts.column}>
            {initialNotice && <Notice kind="info">{initialNotice}</Notice>}
            <div className={styles.pathField}>
              <TextField label="ROM-Datei" showLabel value={romPath} onCommit={(path) => void readRom(path, false)} placeholder="Pfad zur .nds-Datei" error={romError ?? undefined} />
            </div>
            <Button kind="secondary" icon="open" onClick={() => void pickRom()}>
              Datei wählen…
            </Button>
            {reading && <p className={parts.note}>Lese die ROM …</p>}
            <p className={parts.note}>{OWN_ROM}</p>
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
      title: "Vorlage",
      canNext: true,
      content: (
        <section className={`${parts.column} ${parts.wide}`}>
          <TemplateCards value={template} untested={rom?.support.kind === "untested"} onChange={setTemplate} />
        </section>
      ),
    },
    {
      id: "dest",
      title: "Name und Ziel",
      canNext: problem === null && destOk,
      content: (
        <>
          <section className={parts.column}>
            <div className={styles.pathField}>
              <TextField
                label="Name"
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
              <TextField label="Ordner" showLabel value={parent} onCommit={(typed) => {
                  setParentError(null);
                  setParent(typed.trim());
                }} error={folderError} />
            </div>
            <Button kind="secondary" icon="open" onClick={() => void pickFolder()}>
              Ordner wählen…
            </Button>
          </section>
          <section className={`${parts.column} ${parts.wide}`}>
            <Switch label="Endung _DSPRE_contents (für DSPRE)" checked={dspre} onChange={setDspre} />
            <div className={parts.group}>
              <span className={parts.label}>Neues Projekt</span>
              <p className={styles.path}>{dest ?? "–"}</p>
            </div>
            {destNote && <p className={parts.note}>{destNote}</p>}
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: ready,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>Das neue Projekt</h3>
            <dl className={styles.facts}>
              <div>
                <dt>ROM</dt>
                <dd className={styles.code}>{romPath}</dd>
              </div>
              <div>
                <dt>Spiel</dt>
                <dd>{game}</dd>
              </div>
              <div>
                <dt>Vorlage</dt>
                <dd>{TEMPLATE_NAMES[template]}</dd>
              </div>
              <div>
                <dt>Ordner</dt>
                <dd className={styles.code}>{dest ?? "–"}</dd>
              </div>
            </dl>
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Ablauf</h3>
            <div className={styles.progress}>
              <CheckList
                ariaLabel="Ablauf"
                rows={phases.map((r) => ({
                  id: r.id,
                  state: CHECK_STATE[r.state],
                  text: r.label,
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

  const result = created ? (
    <DoneResult
      title="Projekt angelegt"
      lines={[created.root, `Basis-ROM SHA-1 ${created.sha1.slice(0, 8)}…`]}
      next={[created.template === "data_only" ? "Öffnen startet im Bereich Pokémon." : "Öffnen lädt das Projekt."]}
    />
  ) : null;

  return (
    <WizardFrame
      title="Neues Spiel aus ROM"
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
      submitLabel="Anlegen"
      onSubmit={() => void create()}
      onCancel={onCancel}
      onShow={created ? () => onOpen(created) : undefined}
      showLabel="Öffnen"
    />
  );
}
