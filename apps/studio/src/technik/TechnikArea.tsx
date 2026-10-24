import { useId, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { HeaderPatchDto } from "../api";
import { toolsText } from "../app/toolStatus";
import type { ApiculaState, LspTool, LspToolState, ToolStatusStore } from "../app/toolStatus";
import { EmulatorRow } from "../ui/frame/EmulatorRow";
import { ListPane } from "../ui/frame/ListPane";
import type { Depth } from "../ui/frame/shellState";
import { ago, useNow } from "../ui/frame/timeAgo";
import { Badge, Button, Dialog, EmptyState, ListRow, Notice, useRovingList } from "../ui/primitives";
import type { BadgeKind, IconName } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { ProblemsTable } from "../workbench/problems/ProblemsPanel";
import { countsText } from "../workbench/problems/problems";
import type { ProblemsStore } from "../workbench/problems/problems";
import { useHeaderPatch } from "../workbench/wizards/useHeaderPatch";
import styles from "./TechnikArea.module.css";

/** The entries of the list, each with its surface. */
export type TechnikEntry = "bauen" | "patches" | "emulator" | "werkzeuge";

export type TechnikAreaProps = {
  /** The open project's folder; null without one. */
  project: string | null;
  entry: TechnikEntry;
  onEntry: (entry: TechnikEntry) => void;
  /** Whether the list shows or is folded to its grip. */
  listOpen: boolean;
  onToggleList: () => void;
  problems: ProblemsStore;
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
};

const ENTRIES: readonly { id: TechnikEntry; label: string; icon: IconName }[] = [
  { id: "bauen", label: "Prüfen und Bauen", icon: "build" },
  { id: "patches", label: "Patches", icon: "patch" },
  { id: "emulator", label: "Emulator", icon: "game" },
  { id: "werkzeuge", label: "Werkzeuge", icon: "tools" },
];

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
 * The area Technik (spec 5.9): a list of four entries with their state, and the surface of the chosen one. Prüfen und Bauen
 * and Patches need a project; Emulator and Werkzeuge do not. The patch state is read once per project, when the area shows.
 */
export function TechnikArea(props: TechnikAreaProps) {
  return props.project === null ? <TechnikView {...props} patch={null} /> : <WithPatch key={props.project} {...props} />;
}

function WithPatch(props: TechnikAreaProps) {
  const patch = useHeaderPatch(props.onHeaderPatch, { repaired: props.onHeaderPatchRepaired });
  return <TechnikView {...props} patch={patch} />;
}

function TechnikView({ patch, ...props }: TechnikAreaProps & { patch: HeaderPatch | null }) {
  const { entry, onEntry, listOpen, onToggleList, lastBuiltAt, emulatorPath, tools } = props;
  const status = useSyncExternalStore(tools.subscribe, tools.get);
  const now = useNow();
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const subtitles: Record<TechnikEntry, string> = {
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
      {entry === "bauen" ? (
        <BuildSurface {...props} now={now} />
      ) : entry === "patches" ? (
        <PatchSurface patch={patch} onOpenProject={props.onOpenProject} />
      ) : entry === "emulator" ? (
        <Surface entry="emulator" title="Emulator">
          <p className={styles.text}>Das Studio startet gebaute ROMs in diesem Emulator. Die Wahl gilt für alle Projekte.</p>
          <EmulatorRow path={emulatorPath} onPick={props.onPickEmulator} />
        </Surface>
      ) : (
        <ToolsSurface lsp={status.lsp} apicula={status.apicula} subtitle={subtitles.werkzeuge} />
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

function BuildSurface({ project, problems, depth, goTo, busy, lastBuiltAt, lastRom, onBuild, onRun, canRun, onOpenProject, now }: TechnikAreaProps & { now: number }) {
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
      {rows.length === 0 ? (
        <p className={styles.text}>Keine Probleme. Beim Speichern und Bauen landen Fehler und Warnungen hier.</p>
      ) : (
        <div className={styles.table}>
          <ProblemsTable rows={rows} depth={depth} goTo={goTo} />
        </div>
      )}
    </Surface>
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
        {patch.error !== null && <Notice kind="error">{patch.error}</Notice>}
      </section>
      <Dialog
        open={asking}
        onOpenChange={setAsking}
        title="Patch „Dynamische Header“ anwenden?"
        description="Der Patch ändert Bytes im Spielcode (arm9.bin), die Sicherung arm9.bin.backup wird angelegt. Ungespeicherte Header-Änderungen erst speichern."
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

function ToolsSurface({ lsp, apicula, subtitle }: { lsp: LspTool | null; apicula: ApiculaState; subtitle: string }) {
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
      </ul>
    </Surface>
  );
}
