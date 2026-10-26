import { useId, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { HeaderPatchDto } from "../api";
import { toolsText } from "../app/toolStatus";
import type { ApiculaState, LspTool, LspToolState, ToolStatusStore } from "../app/toolStatus";
import { CHECK_CODES, CHECK_MESSAGES } from "../checks/messages";
import type { ChecksApi } from "../checks/useChecks";
import { useProjectPreview } from "../project/useProjectPreview";
import type { ProjectPreviewState } from "../project/useProjectPreview";
import { EmulatorRow } from "../ui/frame/EmulatorRow";
import { ListPane } from "../ui/frame/ListPane";
import { ProjectPreview } from "../ui/frame/ProjectPreview";
import type { Depth } from "../ui/frame/shellState";
import { ago, useNow } from "../ui/frame/timeAgo";
import { Badge, Button, Dialog, EmptyState, ListRow, Notice, Section, Switch, useRovingList } from "../ui/primitives";
import type { BadgeKind, IconName } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { ChecksFoot, ChecksNotes, ProblemsTable, UNCHECKED_WHY, UncheckedNotice, checksPending } from "../workbench/problems/ProblemsPanel";
import { countsText, isFinding } from "../workbench/problems/problems";
import type { ProblemRow, ProblemsStore } from "../workbench/problems/problems";
import { useHeaderPatch } from "../workbench/wizards/useHeaderPatch";
import styles from "./TechnikArea.module.css";

/** The entries of the list, each with its surface. */
export type TechnikEntry = "projekt" | "bauen" | "patches" | "emulator" | "werkzeuge";

export type TechnikAreaProps = {
  /** The open project's folder; null without one. */
  project: string | null;
  entry: TechnikEntry;
  onEntry: (entry: TechnikEntry) => void;
  /** Whether the list shows or is folded to its grip. */
  listOpen: boolean;
  onToggleList: () => void;
  problems: ProblemsStore;
  /** The checks of the project: the same table as the Problems panel, „Ausgangslage übernehmen“ and a switch per check. Null: none. */
  checks: ChecksApi | null;
  /** Tüfteln shows the problems' file paths. */
  depth: Depth;
  /** Follows a problem's link. */
  goTo: (target: NavTarget) => void;
  /** A build or a save runs. */
  busy: boolean;
  /** When the last ROM was built in this session, and where it went. */
  lastBuiltAt: number | null;
  lastRom: string | null;
  /** Builds the ROM (asks where to). */
  onBuild: () => void;
  /** Starts the last ROM built in the emulator. */
  onRun: () => void;
  /** Whether an emulator can be started here (not in the browser mode: the bridge refuses). */
  canRun: boolean;
  onOpenProject: () => void;
  emulatorPath: string | null;
  onPickEmulator: () => void;
  tools: ToolStatusStore;
  /** After the patch went in: the header table is read again. */
  onHeaderPatch: () => Promise<void>;
  /** A repair of the patch went in (the app closes the toast that offered it and says what changed). */
  onHeaderPatchRepaired?: () => void;
  /** "Welt leeren…" (Werkzeuge): opens the wizard that makes the world an empty region with the start town. */
  onClearWorld: () => void;
  /**
   * The name the world gives the header of the start town (the area's name in Bauen), for the hint below the preview picture; null when
   * it has none. Without it the hint names no town.
   */
  startTown?: (header: number) => string | null;
};

const ENTRIES: readonly { id: TechnikEntry; label: string; icon: IconName }[] = [
  { id: "projekt", label: "Projekt", icon: "open" },
  { id: "bauen", label: "Prüfen und Bauen", icon: "build" },
  { id: "patches", label: "Patches", icon: "patch" },
  { id: "emulator", label: "Emulator", icon: "game" },
  { id: "werkzeuge", label: "Werkzeuge", icon: "tools" },
];

const PREVIEW_SUBTITLE = { eigen: "Eigenes Vorschaubild", standard: "Kartenbild der Startstadt" } as const;

function previewSubtitle(state: ProjectPreviewState | null): string {
  if (state === null) return "Kein Projekt geöffnet";
  if (state.preview === null) return "Wird gelesen …";
  const { source } = state.preview;
  return source === null ? "Kein Vorschaubild" : PREVIEW_SUBTITLE[source];
}

/** The last part of a folder path: the project's name. */
const folderName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

/** The Dynamic Headers patch as the area works on it; null without a project. */
type HeaderPatch = ReturnType<typeof useHeaderPatch>;

type PatchState = "applied" | "available" | "broken" | "unsupported";

function patchState(patch: HeaderPatchDto): PatchState {
  if (!patch.supported) return "unsupported";
  if (patch.repairable) return "broken";
  return patch.applied ? "applied" : "available";
}

const PATCH_PLAQUE: Record<PatchState, { kind: BadgeKind; text: string }> = {
  applied: { kind: "success", text: "Angewendet" },
  available: { kind: "info", text: "Verfügbar" },
  broken: { kind: "error", text: "Fehlerhaft" },
  unsupported: { kind: "ptHgssOnly", text: "Nur Platin und HGSS" },
};

const PATCH_SUBTITLE: Record<PatchState, string> = {
  applied: "Dynamische Header angewendet",
  available: "Dynamische Header verfügbar",
  broken: "Dynamische Header fehlerhaft",
  unsupported: "Nur Platin und HGSS",
};

function patchSubtitle(patch: HeaderPatch | null): string {
  if (patch === null) return "Kein Projekt geöffnet";
  if (patch.patch !== null) return PATCH_SUBTITLE[patchState(patch.patch)];
  return patch.error === null ? "Wird gelesen …" : "Nicht lesbar";
}

/**
 * The area Technik (spec 5.9, phase 4.0 §3.5): a list of five entries with their state, and the surface of the chosen one. Projekt,
 * Prüfen und Bauen and Patches need a project; Emulator and Werkzeuge do not (only „Welt leeren“ in Werkzeuge does). The patch state and
 * the preview picture are read once per project, when the area shows.
 */
export function TechnikArea(props: TechnikAreaProps) {
  return props.project === null ? <TechnikView {...props} patch={null} preview={null} /> : <WithProject key={props.project} {...props} />;
}

function WithProject(props: TechnikAreaProps) {
  const patch = useHeaderPatch(props.onHeaderPatch, { repaired: props.onHeaderPatchRepaired });
  const preview = useProjectPreview();
  return <TechnikView {...props} patch={patch} preview={preview} />;
}

function TechnikView({ patch, preview, ...props }: TechnikAreaProps & { patch: HeaderPatch | null; preview: ProjectPreviewState | null }) {
  const { entry, onEntry, listOpen, onToggleList, lastBuiltAt, emulatorPath, tools } = props;
  const status = useSyncExternalStore(tools.subscribe, tools.get);
  const now = useNow();
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const subtitles: Record<TechnikEntry, string> = {
    projekt: previewSubtitle(preview),
    bauen: lastBuiltAt === null ? "Noch nicht gebaut" : `gebaut ${ago(lastBuiltAt, now)}`,
    patches: patchSubtitle(patch),
    emulator: emulatorPath ?? "Kein Emulator gewählt",
    werkzeuge: toolsText(status),
  };
  return (
    <div className={styles.area}>
      <ListPane title="Technik" open={listOpen} onToggle={onToggleList}>
        <div ref={list} role="listbox" aria-label="Technik" className={styles.list}>
          {ENTRIES.map((e) => (
            <ListRow key={e.id} label={e.label} icon={e.icon} subtitle={subtitles[e.id]} selected={e.id === entry} onSelect={() => onEntry(e.id)} />
          ))}
        </div>
      </ListPane>
      {entry === "projekt" ? (
        <ProjectSurface project={props.project} preview={preview} startTown={props.startTown} onOpenProject={props.onOpenProject} />
      ) : entry === "bauen" ? (
        <BuildSurface {...props} now={now} />
      ) : entry === "patches" ? (
        <PatchSurface patch={patch} onOpenProject={props.onOpenProject} />
      ) : entry === "emulator" ? (
        <Surface entry="emulator" title="Emulator">
          <p className={styles.text}>Das Studio startet gebaute ROMs in diesem Emulator. Die Wahl gilt für alle Projekte.</p>
          <EmulatorRow path={emulatorPath} onPick={props.onPickEmulator} />
        </Surface>
      ) : (
        <ToolsSurface lsp={status.lsp} apicula={status.apicula} subtitle={subtitles.werkzeuge} hasProject={props.project !== null} onClearWorld={props.onClearWorld} />
      )}
    </div>
  );
}

/** A surface: its title (the entry's name) and what it holds. `data-technik` names the entry. */
function Surface({ entry, title, subtitle, children }: { entry: TechnikEntry; title: string; subtitle?: string; children: ReactNode }) {
  const id = useId();
  return (
    <section className={styles.surface} data-technik={entry} aria-labelledby={id}>
      <header className={styles.head}>
        <h1 id={id} className={styles.title}>
          {title}
        </h1>
        {subtitle && <p className={styles.subtitle}>{subtitle}</p>}
      </header>
      {children}
    </section>
  );
}

function NoProject({ why, onOpenProject }: { why: string; onOpenProject: () => void }) {
  return <EmptyState what="Kein Projekt geöffnet." why={why} action={{ label: "Projekt öffnen", onClick: onOpenProject }} />;
}

type ProjectSurfaceProps = { project: string | null; preview: ProjectPreviewState | null; startTown?: (header: number) => string | null; onOpenProject: () => void };

/**
 * Projekt (phase 4.0 §3.5): the section „Vorschaubild“ with the picture Start shows on the project's card, on a DS screen of 256 × 192 (the
 * project's name on the empty screen while there is none), „Vorschaubild wählen…“ and, with an own picture, „Entfernen“; below them the
 * hint which picture Start shows without an own one, naming the start town when the world knows its name.
 */
function ProjectSurface({ project, preview, startTown, onOpenProject }: ProjectSurfaceProps) {
  if (project === null || preview === null) {
    return (
      <Surface entry="projekt" title="Projekt">
        <NoProject why="Öffne ein Projekt, dann zeigt Technik hier sein Vorschaubild." onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  const name = folderName(project);
  const header = preview.preview?.standardMap?.header;
  const town = header === undefined ? null : (startTown?.(header) ?? null);
  return (
    <Surface entry="projekt" title="Projekt" subtitle={name}>
      <PreviewSection name={name} preview={preview} town={town} />
    </Surface>
  );
}

/**
 * The section „Vorschaubild“ of Technik › Projekt (also in the gallery, with example pictures): the picture on a DS screen of 256 × 192
 * (`name` on the empty screen while there is none), „Vorschaubild wählen…“, „Entfernen“ with an own picture, and the hint naming `town`.
 */
export function PreviewSection({ name, preview, town }: { name: string; preview: ProjectPreviewState; town: string | null }) {
  const headingId = useId();
  const dto = preview.preview;
  return (
    <section className={styles.card} data-section="vorschaubild" aria-labelledby={headingId}>
      <h2 id={headingId} className={styles.heading}>
        Vorschaubild
      </h2>
      <div className={styles.preview}>
        <ProjectPreview name={name} png={dto?.png ?? null} />
      </div>
      <div className={styles.actions}>
        <Button kind="secondary" disabled={preview.busy} onClick={() => preview.change("choose")}>
          Vorschaubild wählen…
        </Button>
        {dto?.source === "eigen" && (
          <Button kind="secondary" disabled={preview.busy} onClick={() => preview.change("remove")}>
            Entfernen
          </Button>
        )}
      </div>
      <p className={styles.note} data-hint="">
        Ohne eigenes Bild zeigt Start das Kartenbild {town === null ? "der Startstadt" : `von ${town}`}.
      </p>
    </section>
  );
}

function BuildSurface({ project, problems, checks, depth, goTo, busy, lastBuiltAt, lastRom, onBuild, onRun, canRun, onOpenProject, now }: TechnikAreaProps & { now: number }) {
  const rows = useSyncExternalStore(problems.subscribe, problems.list);
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  if (project === null) {
    return (
      <Surface entry="bauen" title="Prüfen und Bauen">
        <NoProject why="Öffne ein Projekt, dann prüft und baut Technik es hier." onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  return (
    <Surface entry="bauen" title="Prüfen und Bauen" subtitle={countsText(counts)}>
      <div className={styles.actions}>
        <Button kind="primary" icon="build" disabled={busy} onClick={onBuild}>
          Bauen
        </Button>
        <Button kind="secondary" icon="play" disabled={busy || !canRun || lastRom === null} onClick={onRun} title={canRun && lastRom === null ? "Erst bauen, dann starten" : undefined}>
          Starten
        </Button>
        {busy ? <span className={styles.note}>Baue ROM …</span> : !canRun && <span className={styles.note}>Im Browser-Modus wird nur gebaut</span>}
      </div>
      <dl className={styles.facts}>
        <div className={styles.fact}>
          <dt>Letzter Build</dt>
          <dd>{lastBuiltAt === null ? "Noch nicht gebaut" : ago(lastBuiltAt, now)}</dd>
        </div>
        {lastRom !== null && (
          <div className={styles.fact}>
            <dt>Pfad</dt>
            <dd className={styles.path} title={lastRom}>
              {lastRom}
            </dd>
          </div>
        )}
      </dl>
      <h2 className={styles.heading}>Probleme</h2>
      {checks !== null && <Baseline checks={checks} rows={rows} />}
      {rows.length === 0 && checks === null ? (
        <p className={styles.text}>Keine Probleme. Beim Speichern und Bauen landen Fehler und Warnungen hier.</p>
      ) : (
        <div className={styles.table}>
          {checks !== null && <ChecksNotes checks={checks} depth={depth} />}
          {rows.length === 0 ? (
            <EmptyList checks={checks} />
          ) : (
            <>
              {checks !== null && <UncheckedNotice checks={checks} />}
              <ProblemsTable rows={rows} depth={depth} goTo={goTo} checks={checks} />
            </>
          )}
          {checks !== null && <ChecksFoot checks={checks} depth={depth} />}
        </div>
      )}
      {checks !== null && <CheckSwitches checks={checks} />}
    </Surface>
  );
}

/** The empty list of Prüfen und Bauen: the first run is under way, the checks have no result, or there are no problems. */
function EmptyList({ checks }: { checks: ChecksApi | null }) {
  const view = useSyncExternalStore(checks?.feed.subscribe ?? noSubscribe, checks?.feed.get ?? noView);
  const pending = checksPending(view);
  const text =
    pending === "first"
      ? "Prüft das Projekt … Die Befunde der Prüfungen erscheinen hier, sobald sie fertig sind."
      : pending === "unchecked"
        ? `Nicht geprüft. ${UNCHECKED_WHY}`
        : "Keine Probleme. Die Prüfungen laufen nach jeder Änderung; beim Speichern und Bauen landen Fehler und Warnungen hier.";
  return <p className={`${styles.text} ${styles.inset}`}>{text}</p>;
}

const noSubscribe = () => () => {};
const noView = () => null;

/**
 * „Ausgangslage übernehmen…“ (spec 3b.4 §7): asks first, naming how many warnings and infos it ignores, then ignores every warning and
 * info of the current state. Errors stay.
 */
function Baseline({ checks, rows }: { checks: ChecksApi; rows: readonly ProblemRow[] }) {
  const [asking, setAsking] = useState(false);
  const n = rows.filter((row) => isFinding(row) && row.severity !== "error").length;
  return (
    <div className={styles.actions}>
      <Button kind="secondary" onClick={() => setAsking(true)}>
        Ausgangslage übernehmen…
      </Button>
      <span className={styles.note}>Blendet alle Warnungen und Hinweise von jetzt aus; Fehler bleiben sichtbar.</span>
      <Dialog
        open={asking}
        onOpenChange={setAsking}
        title="Ausgangslage übernehmen?"
        description={`${n} ${n === 1 ? "Warnung oder Hinweis wird" : "Warnungen und Hinweise werden"} ausgeblendet. Ändert sich eine Stelle, meldet sie sich wieder; unter „Wieder melden…“ holst du sie zurück.`}
        actions={[
          { label: "Abbrechen", onClick: () => setAsking(false) },
          {
            label: "Übernehmen",
            kind: "primary",
            onClick: () => {
              setAsking(false);
              void checks.acceptBaseline();
            },
          },
        ]}
      />
    </div>
  );
}

/** A switch per check (folded until asked for); a check that reports errors cannot be switched off. */
function CheckSwitches({ checks }: { checks: ChecksApi }) {
  const [open, setOpen] = useState(false);
  const view = useSyncExternalStore(checks.feed.subscribe, checks.feed.get);
  return (
    <div className={styles.switches}>
      <Section title="Prüfungen ein- und ausschalten" open={open} onToggle={() => setOpen(!open)}>
        <ul role="list" aria-label="Prüfungen ein- und ausschalten" className={styles.switchList}>
          {CHECK_CODES.map((code) => {
            const m = CHECK_MESSAGES[code]!;
            const errors = m.severity === "error";
            return (
              <li key={code} className={styles.switchRow} data-check={code}>
                <Switch label={m.title} checked={!view.disabled.includes(code)} disabled={errors} onChange={(on) => void checks.setEnabled(code, on)} />
                <span className={styles.code}>{code}</span>
                {errors && <span className={styles.note}>Fehler lassen sich nicht abschalten.</span>}
              </li>
            );
          })}
        </ul>
      </Section>
    </div>
  );
}

function PatchSurface({ patch, onOpenProject }: { patch: HeaderPatch | null; onOpenProject: () => void }) {
  const [asking, setAsking] = useState(false);
  const headingId = useId();
  if (patch === null) {
    return (
      <Surface entry="patches" title="Patches">
        <NoProject why="Öffne ein Projekt, dann zeigt Technik hier seine Patches." onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  const state = patch.patch === null ? null : patchState(patch.patch);
  const plaque = state === null ? null : PATCH_PLAQUE[state];
  return (
    <Surface entry="patches" title="Patches">
      <section className={styles.card} aria-labelledby={headingId}>
        <div className={styles.cardHead}>
          <h2 id={headingId} className={styles.heading}>
            Dynamische Header
          </h2>
          {plaque && state && (
            <span data-patch={state}>
              <Badge kind={plaque.kind}>{plaque.text}</Badge>
            </span>
          )}
        </div>
        <p className={styles.text}>
          Neue Orte brauchen eigene Header. Der Patch verlegt die Header in eine Datei je Header und ändert dafür Bytes im Spielcode (arm9.bin). DSPRE kennt ihn.
        </p>
        {patch.patch === null && patch.error === null && <p className={styles.note}>Prüfe den Patch …</p>}
        {state === "broken" && (
          <p className={styles.text}>Der Patch ruft im Spielcode eine falsche Funktion auf: das Spiel bleibt nach dem Intro schwarz. Die Reparatur ändert 4 Bytes in arm9.bin.</p>
        )}
        {(state === "available" || state === "broken") && (
          <div className={styles.actions}>
            {state === "available" ? (
              <Button kind="primary" disabled={patch.busy} onClick={() => setAsking(true)}>
                Patch anwenden
              </Button>
            ) : (
              <Button kind="primary" disabled={patch.busy} onClick={() => void patch.repair()}>
                Patch reparieren
              </Button>
            )}
            {patch.busy && <span className={styles.note}>{state === "available" ? "Wende an …" : "Repariere …"}</span>}
          </div>
        )}
        {patch.snapshot !== null && <p className={styles.text}>Sicherung des alten Ordners: {patch.snapshot}</p>}
        {patch.error !== null && <Notice kind="error">{patch.error}</Notice>}
      </section>
      <Dialog
        open={asking}
        onOpenChange={setAsking}
        title="Patch „Dynamische Header“ anwenden?"
        description="Der Patch ändert Bytes im Spielcode (arm9.bin), die Sicherung arm9.bin.backup wird angelegt. Ungespeicherte Änderungen an Gebieten und Räumen bleiben ungespeichert."
        actions={[
          { label: "Abbrechen", onClick: () => setAsking(false) },
          {
            label: "Patch anwenden",
            kind: "primary",
            onClick: () => {
              setAsking(false);
              void patch.apply();
            },
          },
        ]}
      >
        <Badge kind="irreversible" />
      </Dialog>
    </Surface>
  );
}

const LSP_BADGE: Record<LspToolState, { kind: BadgeKind; text: string }> = {
  aus: { kind: "neutral", text: "Aus" },
  startet: { kind: "neutral", text: "Startet" },
  bereit: { kind: "success", text: "Bereit" },
  fehlt: { kind: "warning", text: "Fehlt" },
  fehler: { kind: "error", text: "Fehler" },
};

const APICULA_BADGE: Record<ApiculaState, { kind: BadgeKind; text: string; note?: string }> = {
  unbekannt: { kind: "neutral", text: "Unbekannt", note: "Zeigt sich beim ersten Kartenbild." },
  bereit: { kind: "success", text: "Bereit" },
  fehlt: { kind: "warning", text: "Fehlt", note: "Ohne apicula gibt es keine Kartenbilder und 3D-Modelle." },
};

function lspNote(lsp: LspTool | null): string | undefined {
  if (lsp === null) return "Startet, sobald das Panel Script einmal gezeigt wird.";
  // Off: the script panel says why when it knows (scripts not adopted); without a reason the note stays neutral.
  if (lsp.state === "aus") return lsp.note ?? "Nicht gestartet.";
  return lsp.note;
}

type ToolsProps = { lsp: LspTool | null; apicula: ApiculaState; subtitle: string; hasProject: boolean; onClearWorld: () => void };

function ToolsSurface({ lsp, apicula, subtitle, hasProject, onClearWorld }: ToolsProps) {
  const lspBadge = lsp === null ? { kind: "neutral" as const, text: "Nicht gestartet" } : LSP_BADGE[lsp.state];
  const apiculaBadge = APICULA_BADGE[apicula];
  const note = lspNote(lsp);
  return (
    <Surface entry="werkzeuge" title="Werkzeuge" subtitle={subtitle}>
      <ul className={styles.tools}>
        <li className={styles.tool} data-tool="apicula">
          <div className={styles.toolHead}>
            <span className={styles.toolName}>apicula</span>
            <Badge kind={apiculaBadge.kind}>{apiculaBadge.text}</Badge>
          </div>
          <p className={styles.text}>Wandelt die Karten des Spiels in 3D-Modelle und Kartenbilder.</p>
          {apiculaBadge.note && <p className={styles.note}>{apiculaBadge.note}</p>}
        </li>
        <li className={styles.tool} data-tool="rotom-lsp" data-lsp={lsp?.state ?? "none"}>
          <div className={styles.toolHead}>
            <span className={styles.toolName}>rotom-lsp</span>
            <Badge kind={lspBadge.kind}>{lspBadge.text}</Badge>
            {lsp?.state === "fehler" && lsp.restart && (
              <Button kind="quiet" size="s" onClick={lsp.restart}>
                Neu starten
              </Button>
            )}
          </div>
          <p className={styles.text}>Sprachdienst für Scripts: Vervollständigung und Diagnosen im Editor.</p>
          {note && <p className={styles.note}>{note}</p>}
        </li>
        <li className={styles.tool} data-tool="welt-leeren">
          <div className={styles.toolHead}>
            <span className={styles.toolTitle}>Welt leeren</span>
          </div>
          <p className={styles.text}>Macht aus der Welt eine leere Region mit Startstadt. Die Welt holst du mit ⌘Z zurück.</p>
          <div className={styles.actions}>
            <Button kind="secondary" icon="world" disabled={!hasProject} onClick={onClearWorld}>
              Welt leeren…
            </Button>
            {!hasProject && <span className={styles.note}>Erst ein Projekt öffnen</span>}
          </div>
        </li>
      </ul>
    </Surface>
  );
}
