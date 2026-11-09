import { useEffect, useId, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { Against, AssetId, BuildStep, ChangedEntry, DiffSubject, HeaderPatchDto, LiveStatusDto } from "../api";
import { toolsText } from "../app/toolStatus";
import type { ApiculaState, LspTool, LspToolState, ToolStatusStore } from "../app/toolStatus";
import { CHECK_CODES, checkMessage } from "../checks/messages";
import type { ChecksApi } from "../checks/useChecks";
import type { GitStatusApi } from "../git/useGitStatus";
import { t } from "../i18n";
import { useReset } from "../original/OriginalContext";
import { useProjectPreview } from "../project/useProjectPreview";
import type { ProjectPreviewState } from "../project/useProjectPreview";
import { EmulatorRow } from "../ui/frame/EmulatorRow";
import { LiveRow } from "../ui/frame/LiveRow";
import { ListPane } from "../ui/frame/ListPane";
import { ProjectPreview } from "../ui/frame/ProjectPreview";
import type { Depth } from "../ui/frame/shellState";
import { ago, useNow } from "../ui/frame/timeAgo";
import { Badge, Button, Dialog, EmptyState, ListRow, Notice, Section, Switch, useRovingList } from "../ui/primitives";
import type { BadgeKind, IconName } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { ChecksFoot, ChecksNotes, ProblemsTable, UncheckedNotice, checksPending, uncheckedWhy } from "../workbench/problems/ProblemsPanel";
import { countsText, isFinding } from "../workbench/problems/problems";
import type { ProblemRow, ProblemsStore } from "../workbench/problems/problems";
import { useHeaderPatch } from "../workbench/wizards/useHeaderPatch";
import { PublishSurface, publishSubtitle } from "./PublishSurface";
import { BuildSteps } from "./BuildSteps";
import { patchBuildNote } from "./stepWords";
import { ChangesSurface, changesSubtitle } from "./ChangesSurface";
import { TextProjectSurface, textProjectSubtitle } from "./TextProjectSurface";
import type { TextProjectView } from "./TextProjectSurface";
import styles from "./TechnikArea.module.css";
import type { BaseApi } from "./useBase";
import type { ChangesApi } from "./useChanges";

/** The entries of the list, each with its surface. */
export type TechnikEntry = "projekt" | "textprojekt" | "bauen" | "veroeffentlichen" | "aenderungen" | "patches" | "emulator" | "werkzeuge";

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
  /** The steps of the last build in this session; null before the first. */
  lastSteps: BuildStep[] | null;
  /** The last build was started with „Neu bauen (ohne Cache)“ (its steps say so instead of „erster Build“). */
  lastBuildFresh?: boolean;
  /** The last steps are those of a release („Veröffentlichen“), which built this ROM (the facts above keep naming the ROM of Bauen); null for a normal build. */
  lastPatchRom?: string | null;
  /** Builds the ROM (asks where to). */
  onBuild: () => void;
  /** „Neu bauen (ohne Cache)“: drops the build state, then builds every step fully (asks where to). */
  onBuildFresh: () => void;
  /** Starts the last ROM built in the emulator. */
  onRun: () => void;
  /** Whether an emulator can be started here (not in the browser mode: the bridge refuses). */
  canRun: boolean;
  onOpenProject: () => void;
  emulatorPath: string | null;
  onPickEmulator: () => void;
  /** Play mode's status (Emulator: the row „Spielmodus live“ reads its melonDS); absent: none known. */
  live?: LiveStatusDto;
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
  /** The base ROM and the releases (Veröffentlichen), held by the app above the areas; null without a project. */
  base: BaseApi | null;
  /** „Veröffentlichen…“ (Veröffentlichen): the app opens the wizard. */
  onPublish: () => void;
  /** „Im Finder zeigen“ of a release file (Veröffentlichen). */
  onReveal: (path: string) => Promise<void>;
  /** What of the saved project differs from the base ROM (Änderungen), held by the app; null without a project. */
  changes: ChangesApi | null;
  /** The store's assets with unsaved edits: Änderungen marks their rows „Ungespeichert“. */
  dirty: readonly AssetId[];
  /** The project as text project and its sync with DSPRE (Textprojekt), held by the app; null without a project. */
  textProject: TextProjectView | null;
  /** What Änderungen compares against; the app keeps the choice for the session. */
  against: Against;
  onAgainst: (against: Against) => void;
  /** The Git state of the project (Änderungen against the last commit), held by the app; null without a project. */
  git: GitStatusApi | null;
  /** The unsaved edits as entities (Änderungen against the saved state); null while they are read. */
  unsaved: readonly ChangedEntry[] | null;
  /** Why the last read of the unsaved edits failed; null when it went through. */
  unsavedError?: string | null;
  /** Reads the unsaved edits again (after a failed read). */
  onReloadUnsaved?: () => void;
  /** „Git einrichten…“ (Änderungen against the last commit without a repository): the app's dialog. */
  onGitSetup: () => void;
  /** „Stand sichern…“ (Änderungen against the last commit, in the project's own repository): the app's dialog. */
  onCommitAll: () => void;
  /** Moves whenever the compared state may differ (a save, a reset, an edit): the detail of Änderungen reads its diff again. */
  revision: number;
  /** The entity chosen in Änderungen, held by the app for the session (it survives „Öffnen“ and the way back); without it the surface holds its own. */
  changesChosen?: DiffSubject | null;
  onChangesChosen?: (subject: DiffSubject | null) => void;
};

/** The entries of the list: their label is `technikArea.entries.<id>` in the catalog. */
const ENTRIES: readonly { id: TechnikEntry; icon: IconName }[] = [
  { id: "projekt", icon: "open" },
  { id: "textprojekt", icon: "textproject" },
  { id: "bauen", icon: "build" },
  { id: "veroeffentlichen", icon: "publish" },
  { id: "aenderungen", icon: "changes" },
  { id: "patches", icon: "patch" },
  { id: "emulator", icon: "game" },
  { id: "werkzeuge", icon: "tools" },
];

function previewSubtitle(state: ProjectPreviewState | null): string {
  if (state === null) return t("technik:shared.noProject");
  if (state.preview === null) return t("technik:shared.reading");
  const { source } = state.preview;
  return source === null ? t("technik:technikArea.noPreview") : t(`technik:technikArea.previewSource.${source}`);
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

/** The colour of the plaque of each state of the patch; its word is `technikArea.patchPlaque.<state>`. */
const PATCH_PLAQUE_KIND: Record<PatchState, BadgeKind> = { applied: "success", available: "info", broken: "error", unsupported: "ptHgssOnly" };

function patchSubtitle(patch: HeaderPatch | null): string {
  if (patch === null) return t("technik:shared.noProject");
  if (patch.patch !== null) return t(`technik:technikArea.patchSubtitle.${patchState(patch.patch)}`);
  return patch.error === null ? t("technik:shared.reading") : t("technik:shared.notReadable");
}

/**
 * The area Technik (spec 5.9, phase 4.0 §3.5): a list of eight entries with their state, and the surface of the chosen one. Projekt,
 * Textprojekt, Prüfen und Bauen, Veröffentlichen, Änderungen and Patches need a project; Emulator and Werkzeuge do not (only „Welt leeren“ in
 * Werkzeuge does). The patch state and the preview picture are read once per project, when the area shows; the base ROM (`base`) and the
 * compare against it (`changes`) come from the app, the status of the base for resetting from `useReset`.
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
  const { t } = useTranslation("technik");
  const { entry, onEntry, listOpen, onToggleList, lastBuiltAt, emulatorPath, tools } = props;
  const status = useSyncExternalStore(tools.subscribe, tools.get);
  const original = useReset().status;
  const now = useNow();
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const subtitles: Record<TechnikEntry, string> = {
    projekt: previewSubtitle(preview),
    textprojekt: textProjectSubtitle(props.textProject),
    bauen: lastBuiltAt === null ? t("technikArea.notBuilt") : t("technikArea.builtAgo", { ago: ago(lastBuiltAt, now) }),
    veroeffentlichen: publishSubtitle(props.base),
    aenderungen: changesSubtitle(props.against, props.changes, original, props.git?.status ?? null, props.unsaved),
    patches: patchSubtitle(patch),
    emulator: emulatorPath ?? t("technikArea.noEmulator"),
    werkzeuge: toolsText(status),
  };
  return (
    <div className={styles.area}>
      <ListPane title={t("technikArea.title")} open={listOpen} onToggle={onToggleList}>
        <div ref={list} role="listbox" aria-label={t("technikArea.title")} className={styles.list}>
          {ENTRIES.map((e) => (
            <ListRow key={e.id} label={t(`technikArea.entries.${e.id}`)} icon={e.icon} subtitle={subtitles[e.id]} selected={e.id === entry} onSelect={() => onEntry(e.id)} />
          ))}
        </div>
      </ListPane>
      {entry === "projekt" ? (
        <ProjectSurface project={props.project} preview={preview} startTown={props.startTown} onOpenProject={props.onOpenProject} />
      ) : entry === "textprojekt" ? (
        <TextProjectSurface text={props.textProject} onOpenProject={props.onOpenProject} depth={props.depth} />
      ) : entry === "bauen" ? (
        <BuildSurface {...props} now={now} />
      ) : entry === "veroeffentlichen" ? (
        <PublishSurface base={props.base} building={props.busy} onOpenProject={props.onOpenProject} onPublish={props.onPublish} onReveal={props.onReveal} />
      ) : entry === "aenderungen" ? (
        <ChangesSurface
          changes={props.changes}
          status={original}
          dirty={props.dirty}
          onOpenProject={props.onOpenProject}
          against={props.against}
          onAgainst={props.onAgainst}
          git={props.git}
          unsaved={props.unsaved}
          unsavedError={props.unsavedError}
          onReloadUnsaved={props.onReloadUnsaved}
          onGitSetup={props.onGitSetup}
          onCommitAll={props.onCommitAll}
          goTo={props.goTo}
          revision={props.revision}
          chosen={props.changesChosen}
          onChoose={props.onChangesChosen}
        />
      ) : entry === "patches" ? (
        <PatchSurface patch={patch} onOpenProject={props.onOpenProject} />
      ) : entry === "emulator" ? (
        <Surface entry="emulator" title={t("technikArea.emulatorTitle")}>
          <p className={styles.text}>{t("technikArea.emulatorText")}</p>
          <EmulatorRow path={emulatorPath} onPick={props.onPickEmulator} />
          <LiveRow melonds={props.live?.melonds} emulator={emulatorPath} />
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
  const { t } = useTranslation("technik");
  return <EmptyState what={t("shared.noProjectWhat")} why={why} action={{ label: t("shared.openProject"), onClick: onOpenProject }} />;
}

type ProjectSurfaceProps = { project: string | null; preview: ProjectPreviewState | null; startTown?: (header: number) => string | null; onOpenProject: () => void };

/**
 * Projekt (phase 4.0 §3.5): the section „Vorschaubild“ with the picture Start shows on the project's card, on a DS screen of 256 × 192 (the
 * project's name on the empty screen while there is none), „Vorschaubild wählen…“ and, with an own picture, „Entfernen“; below them the
 * hint which picture Start shows without an own one, naming the start town when the world knows its name.
 */
function ProjectSurface({ project, preview, startTown, onOpenProject }: ProjectSurfaceProps) {
  const { t } = useTranslation("technik");
  if (project === null || preview === null) {
    return (
      <Surface entry="projekt" title={t("technikArea.project.title")}>
        <NoProject why={t("technikArea.project.noProjectWhy")} onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  const name = folderName(project);
  const header = preview.preview?.standardMap?.header;
  const town = header === undefined ? null : (startTown?.(header) ?? null);
  return (
    <Surface entry="projekt" title={t("technikArea.project.title")} subtitle={name}>
      <PreviewSection name={name} preview={preview} town={town} />
    </Surface>
  );
}

/**
 * The section „Vorschaubild“ of Technik › Projekt (also in the gallery, with example pictures): the picture on a DS screen of 256 × 192
 * (`name` on the empty screen while there is none), „Vorschaubild wählen…“, „Entfernen“ with an own picture, and the hint naming `town`.
 */
export function PreviewSection({ name, preview, town }: { name: string; preview: ProjectPreviewState; town: string | null }) {
  const { t } = useTranslation("technik");
  const headingId = useId();
  const dto = preview.preview;
  const choose = useRef<HTMLButtonElement>(null);
  const running = useRef<"choose" | "remove" | null>(null);
  // Busy blocks the actions with aria-disabled, so the focused button keeps focus while the write runs. When it ends, focus rests on
  // „Vorschaubild wählen…“: after choosing (set, cancelled or failed) and after „Entfernen“, whose button is gone then (focus fell to the body).
  useEffect(() => {
    const how = running.current;
    if (preview.busy || how === null) return;
    running.current = null;
    const active = document.activeElement;
    if (how === "choose" || active === null || active === document.body) choose.current?.focus();
  }, [preview.busy]);
  const change = (how: "choose" | "remove") => {
    if (preview.busy) return;
    running.current = how;
    preview.change(how);
  };
  return (
    <section className={styles.card} data-section="vorschaubild" aria-labelledby={headingId}>
      <h2 id={headingId} className={styles.heading}>
        {t("technikArea.preview.heading")}
      </h2>
      <div className={styles.preview}>
        <ProjectPreview name={name} png={dto?.png ?? null} />
      </div>
      <div className={styles.actions}>
        <Button ref={choose} kind="secondary" aria-disabled={preview.busy || undefined} onClick={() => change("choose")}>
          {t("technikArea.preview.choose")}
        </Button>
        {dto?.source === "eigen" && (
          <Button kind="secondary" aria-disabled={preview.busy || undefined} onClick={() => change("remove")}>
            {t("technikArea.preview.remove")}
          </Button>
        )}
      </div>
      <p className={styles.note} data-hint="">
        {town === null ? t("technikArea.preview.hintStartTown") : t("technikArea.preview.hintTown", { town })}
      </p>
    </section>
  );
}

function BuildSurface({ project, problems, checks, depth, goTo, busy, lastBuiltAt, lastRom, lastSteps, lastBuildFresh = false, lastPatchRom = null, onBuild, onBuildFresh, onRun, canRun, onOpenProject, now }: TechnikAreaProps & { now: number }) {
  const { t } = useTranslation("technik");
  const rows = useSyncExternalStore(problems.subscribe, problems.list);
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  if (project === null) {
    return (
      <Surface entry="bauen" title={t("technikArea.build.title")}>
        <NoProject why={t("technikArea.build.noProjectWhy")} onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  // Busy blocks with `aria-disabled` and this guard, not with `disabled`: the button that started the build keeps keyboard focus (and is
  // where the dialog of a text conflict returns it to); a button blocked for another reason stays `disabled`.
  const whenIdle = (action: () => void) => () => {
    if (!busy) action();
  };
  return (
    <Surface entry="bauen" title={t("technikArea.build.title")} subtitle={countsText(counts)}>
      <div className={styles.actions}>
        <Button kind="primary" icon="build" aria-disabled={busy || undefined} onClick={whenIdle(onBuild)}>
          {t("technikArea.build.build")}
        </Button>
        <Button kind="secondary" icon="build" aria-disabled={busy || undefined} onClick={whenIdle(onBuildFresh)} title={t("technikArea.build.rebuildHint")}>
          {t("technikArea.build.rebuild")}
        </Button>
        <Button kind="secondary" icon="play" disabled={!canRun || lastRom === null} aria-disabled={busy || undefined} onClick={whenIdle(onRun)} title={canRun && lastRom === null ? t("technikArea.build.runHint") : undefined}>
          {t("technikArea.build.run")}
        </Button>
        {busy ? <span className={styles.note}>{t("technikArea.build.building")}</span> : !canRun && <span className={styles.note}>{t("technikArea.build.browserMode")}</span>}
      </div>
      <dl className={styles.facts}>
        <div className={styles.fact}>
          <dt>{t("technikArea.build.lastBuild")}</dt>
          <dd>{lastBuiltAt === null ? t("technikArea.notBuilt") : ago(lastBuiltAt, now)}</dd>
        </div>
        {lastRom !== null && (
          <div className={styles.fact}>
            <dt>{t("technikArea.build.path")}</dt>
            <dd className={styles.path} title={lastRom}>
              {lastRom}
            </dd>
          </div>
        )}
      </dl>
      <h2 className={styles.heading}>{t("technikArea.build.stepsHeading")}</h2>
      {lastSteps !== null && lastPatchRom !== null && (
        <p className={styles.text} data-steps-source="">
          {patchBuildNote(lastPatchRom, project)}
        </p>
      )}
      {lastSteps === null ? <p className={styles.text}>{t("technikArea.build.noBuildYet")}</p> : <BuildSteps steps={lastSteps} depth={depth} fresh={lastBuildFresh} />}
      <h2 className={styles.heading}>{t("technikArea.build.problemsHeading")}</h2>
      {checks !== null && <Baseline checks={checks} rows={rows} />}
      {rows.length === 0 && checks === null ? (
        <p className={styles.text}>{t("technikArea.build.noProblems")}</p>
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
  const { t } = useTranslation("technik");
  const view = useSyncExternalStore(checks?.feed.subscribe ?? noSubscribe, checks?.feed.get ?? noView);
  const pending = checksPending(view);
  const text =
    pending === "first"
      ? t("technikArea.build.checking")
      : pending === "unchecked"
        ? t("technikArea.build.unchecked", { why: uncheckedWhy() })
        : t("technikArea.build.noProblemsChecks");
  return <p className={`${styles.text} ${styles.inset}`}>{text}</p>;
}

const noSubscribe = () => () => {};
const noView = () => null;

/**
 * „Ausgangslage übernehmen…“ (spec 3b.4 §7): asks first, naming how many warnings and infos it ignores, then ignores every warning and
 * info of the current state. Errors stay.
 */
function Baseline({ checks, rows }: { checks: ChecksApi; rows: readonly ProblemRow[] }) {
  const { t } = useTranslation("technik");
  const [asking, setAsking] = useState(false);
  const n = rows.filter((row) => isFinding(row) && row.severity !== "error").length;
  return (
    <div className={styles.actions}>
      <Button kind="secondary" onClick={() => setAsking(true)}>
        {t("technikArea.baseline.button")}
      </Button>
      <span className={styles.note}>{t("technikArea.baseline.note")}</span>
      <Dialog
        open={asking}
        onOpenChange={setAsking}
        title={t("technikArea.baseline.title")}
        description={t("technikArea.baseline.description", { count: n })}
        actions={[
          { label: t("shared.cancel"), onClick: () => setAsking(false) },
          {
            label: t("technikArea.baseline.adopt"),
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
  const { t } = useTranslation("technik");
  const [open, setOpen] = useState(false);
  const view = useSyncExternalStore(checks.feed.subscribe, checks.feed.get);
  return (
    <div className={styles.switches}>
      <Section title={t("technikArea.switches.title")} open={open} onToggle={() => setOpen(!open)}>
        <ul role="list" aria-label={t("technikArea.switches.title")} className={styles.switchList}>
          {CHECK_CODES.map((code) => {
            const m = checkMessage(code)!;
            const errors = m.severity === "error";
            return (
              <li key={code} className={styles.switchRow} data-check={code}>
                <Switch label={m.title} checked={!view.disabled.includes(code)} disabled={errors} onChange={(on) => void checks.setEnabled(code, on)} />
                <span className={styles.code}>{code}</span>
                {errors && <span className={styles.note}>{t("technikArea.switches.errorsStay")}</span>}
              </li>
            );
          })}
        </ul>
      </Section>
    </div>
  );
}

function PatchSurface({ patch, onOpenProject }: { patch: HeaderPatch | null; onOpenProject: () => void }) {
  const { t } = useTranslation("technik");
  const [asking, setAsking] = useState(false);
  const headingId = useId();
  if (patch === null) {
    return (
      <Surface entry="patches" title={t("technikArea.patches.title")}>
        <NoProject why={t("technikArea.patches.noProjectWhy")} onOpenProject={onOpenProject} />
      </Surface>
    );
  }
  const state = patch.patch === null ? null : patchState(patch.patch);
  const plaque = state === null ? null : { kind: PATCH_PLAQUE_KIND[state], text: t(`technikArea.patchPlaque.${state}`) };
  return (
    <Surface entry="patches" title={t("technikArea.patches.title")}>
      <section className={styles.card} aria-labelledby={headingId}>
        <div className={styles.cardHead}>
          <h2 id={headingId} className={styles.heading}>
            {t("technikArea.patches.heading")}
          </h2>
          {plaque && state && (
            <span data-patch={state}>
              <Badge kind={plaque.kind}>{plaque.text}</Badge>
            </span>
          )}
        </div>
        <p className={styles.text}>{t("technikArea.patches.text")}</p>
        {patch.patch === null && patch.error === null && <p className={styles.note}>{t("technikArea.patches.checking")}</p>}
        {state === "broken" && <p className={styles.text}>{t("technikArea.patches.brokenText")}</p>}
        {(state === "available" || state === "broken") && (
          <div className={styles.actions}>
            {state === "available" ? (
              <Button kind="primary" disabled={patch.busy} onClick={() => setAsking(true)}>
                {t("technikArea.patches.apply")}
              </Button>
            ) : (
              <Button kind="primary" disabled={patch.busy} onClick={() => void patch.repair()}>
                {t("technikArea.patches.repair")}
              </Button>
            )}
            {patch.busy && <span className={styles.note}>{state === "available" ? t("technikArea.patches.applying") : t("technikArea.patches.repairing")}</span>}
          </div>
        )}
        {patch.snapshot !== null && <p className={styles.text}>{t("technikArea.patches.snapshot", { path: patch.snapshot })}</p>}
        {patch.error !== null && <Notice kind="error">{patch.error}</Notice>}
      </section>
      <Dialog
        open={asking}
        onOpenChange={setAsking}
        title={t("technikArea.patches.dialogTitle")}
        description={t("technikArea.patches.dialogDescription")}
        actions={[
          { label: t("shared.cancel"), onClick: () => setAsking(false) },
          {
            label: t("technikArea.patches.apply"),
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

/** The badges of the tools by state (the colour here, the word as `technikArea.tools.lsp.<state>` and `technikArea.tools.apicula.<state>`). */
const LSP_KIND: Record<LspToolState, BadgeKind> = { aus: "neutral", startet: "neutral", bereit: "success", fehlt: "warning", fehler: "error" };
const APICULA_KIND: Record<ApiculaState, BadgeKind> = { unbekannt: "neutral", bereit: "success", fehlt: "warning" };

/** What apicula's badge adds in the states that have a note. */
const apiculaNote = (state: ApiculaState): string | undefined => (state === "bereit" ? undefined : t(`technik:technikArea.tools.apiculaNote.${state}`));

function lspNote(lsp: LspTool | null): string | undefined {
  if (lsp === null) return t("technik:technikArea.tools.lspNoteNotStarted");
  // Off: the script panel says why when it knows (scripts not adopted); without a reason the note stays neutral.
  if (lsp.state === "aus") return lsp.note ?? t("technik:technikArea.tools.lspOff");
  return lsp.note;
}

type ToolsProps = { lsp: LspTool | null; apicula: ApiculaState; subtitle: string; hasProject: boolean; onClearWorld: () => void };

function ToolsSurface({ lsp, apicula, subtitle, hasProject, onClearWorld }: ToolsProps) {
  const { t } = useTranslation("technik");
  const lspBadge = lsp === null ? { kind: "neutral" as const, text: t("technikArea.tools.lspNotStartedBadge") } : { kind: LSP_KIND[lsp.state], text: t(`technikArea.tools.lsp.${lsp.state}`) };
  const apiculaBadge = { kind: APICULA_KIND[apicula], text: t(`technikArea.tools.apicula.${apicula}`), note: apiculaNote(apicula) };
  const note = lspNote(lsp);
  return (
    <Surface entry="werkzeuge" title={t("technikArea.tools.title")} subtitle={subtitle}>
      <ul className={styles.tools}>
        <li className={styles.tool} data-tool="apicula">
          <div className={styles.toolHead}>
            <span className={styles.toolName}>apicula</span>
            <Badge kind={apiculaBadge.kind}>{apiculaBadge.text}</Badge>
          </div>
          <p className={styles.text}>{t("technikArea.tools.apiculaText")}</p>
          {apiculaBadge.note && <p className={styles.note}>{apiculaBadge.note}</p>}
        </li>
        <li className={styles.tool} data-tool="rotom-lsp" data-lsp={lsp?.state ?? "none"}>
          <div className={styles.toolHead}>
            <span className={styles.toolName}>rotom-lsp</span>
            <Badge kind={lspBadge.kind}>{lspBadge.text}</Badge>
            {lsp?.state === "fehler" && lsp.restart && (
              <Button kind="quiet" size="s" onClick={lsp.restart}>
                {t("technikArea.tools.restart")}
              </Button>
            )}
          </div>
          <p className={styles.text}>{t("technikArea.tools.lspText")}</p>
          {note && <p className={styles.note}>{note}</p>}
        </li>
        <li className={styles.tool} data-tool="welt-leeren">
          <div className={styles.toolHead}>
            <span className={styles.toolTitle}>{t("technikArea.tools.clearWorld")}</span>
          </div>
          <p className={styles.text}>{t("technikArea.tools.clearWorldText")}</p>
          <div className={styles.actions}>
            <Button kind="secondary" icon="world" disabled={!hasProject} onClick={onClearWorld}>
              {t("technikArea.tools.clearWorldButton")}
            </Button>
            {!hasProject && <span className={styles.note}>{t("technikArea.tools.needProject")}</span>}
          </div>
        </li>
      </ul>
    </Surface>
  );
}
