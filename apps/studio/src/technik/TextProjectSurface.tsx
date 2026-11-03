import { useEffect, useId, useRef, useState } from "react";
import type { ReactNode } from "react";
import { conflictDiff, errorText, resolveExternalConflict } from "../api";
import type { AssetId, BrokenEntry, ConflictDiff, ConflictEntry, DeleteTarget, Keep, SyncOutcome, TextProjectStatus } from "../api";
import type { ExternalSyncApi } from "../app/useExternalSync";
import { useDelete } from "../delete/DeleteContext";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { ago, useNow } from "../ui/frame/timeAgo";
import { Badge, Button, EmptyState, Notice, NumberField, Section, SelectField, Table } from "../ui/primitives";
import type { SelectOption, TableColumn } from "../ui/primitives";
import { FIRST_SYNC_NOTE, brokenCountText, conflictCountText, moreText, problemLine, syncProgressText } from "./syncWords";
import styles from "./TextProjectSurface.module.css";

/** What Technik › Textprojekt shows and does for the open project (held by the app). */
export type TextProjectView = {
  /** The folder as a text project; null while it is read. */
  status: TextProjectStatus | null;
  /** The sync with DSPRE (`useExternalSync`). */
  sync: ExternalSyncApi;
  /**
   * A build, a save or a set-up runs: „Jetzt abgleichen“ waits (the build syncs first on its own), „Als Textprojekt einrichten…“ and
   * „Zurücknehmen…“ are locked.
   */
  busy: boolean;
  /** A conflict was resolved: the assets the store loaded anew (the app reads them again and syncs once more). */
  onResolved: (assets: AssetId[]) => void;
  /** „Als Textprojekt einrichten…“ opens its dialog; without it the button is locked. */
  onSetUp?: () => void;
  /** „Zurücknehmen…“ opens its dialog; without it the button is locked. */
  onRevert?: () => void;
  /** An agent sets the folder up as text project (the hub's `busy`): shown, and set-up and take-back are locked meanwhile. */
  agentSetUp?: boolean;
};

/** What the surface says while an agent sets the folder up. */
export const AGENT_SETTING_UP = "Wird von einem Agenten eingerichtet …";

const SETUP_LEAD = "Das Projekt liegt im DSPRE-Layout. Als Textprojekt bekommt jeder Eintrag eine lesbare Datei unter data/, bereit für Git; DSPRE öffnet den Ordner weiter.";

/** The line under „Textprojekt“ in the list of Technik; `text` null: no project. */
export function textProjectSubtitle(text: TextProjectView | null): string {
  if (text === null) return "Kein Projekt geöffnet";
  const { status, sync } = text;
  if (status === null) return "Wird gelesen …";
  switch (status.state) {
    case "dspre":
      return text.agentSetUp ? "Wird eingerichtet …" : "DSPRE-Layout";
    case "older":
      return "Schema veraltet";
    case "newer":
    case "broken":
      return "Nicht lesbar";
    case "current": {
      if (sync.running) return "Gleicht ab …";
      // Conflicts first, then broken files: either is open work, never „Abgeglichen“.
      const conflicts = sync.last?.conflicts.length ?? 0;
      const broken = sync.last?.broken.length ?? 0;
      if (conflicts > 0) return conflictCountText(conflicts);
      return broken > 0 ? brokenCountText(broken) : "Abgeglichen";
    }
  }
}

/** The groups of the surface a message of the sync leads to (`data-sync-…`). */
export type SyncGroup = "conflicts" | "broken" | "taken";

// What the actions of the sync's messages ask for: focus on a group (the heading of the surface when it is not there). The message's
// action navigates here, so the surface may not be shown yet: the request waits for it (`WANTED_FOR`), and the shown surface takes it.
let wanted: { group: SyncGroup; at: number } | null = null;
const wantedListeners = new Set<() => void>();
const WANTED_FOR = 3000;

/** „Anzeigen“ and „Lösen…“ of the sync's messages: the surface puts keyboard focus on the toggle of `group` once it shows (else on its heading). */
export function requestTextProjectFocus(group: SyncGroup) {
  wanted = { group, at: Date.now() };
  wantedListeners.forEach((listener) => listener());
}

export type TextProjectSurfaceProps = {
  /** The text project of the open project; null without a project. */
  text: TextProjectView | null;
  onOpenProject: () => void;
  /** Tüfteln adds „Löschen“ for the entities without a head of their own (default Bauen). */
  depth?: Depth;
};

/**
 * Technik › Textprojekt (4.3b): the folder's state as a text project. A DSPRE folder offers „Als Textprojekt einrichten…“ (locked with the
 * reason when it cannot be set up); a text project names its schema and its last sync, offers „Jetzt abgleichen“ and „Zurücknehmen…“, and
 * lists what the sync found: the conflicts with their diff and a choice each („Auf zwei Seiten geändert“), the files that do not read
 * („Fehlerhafte Dateien“) and what the last takeover brought („Zuletzt übernommen“). Tüfteln adds „Löschen“ (a kind and a number) for the
 * entities no head deletes (matrices, event files, area data, script files, text archives) and for maps.
 */
export function TextProjectSurface({ text, onOpenProject, depth = "bauen" }: TextProjectSurfaceProps) {
  const tinker = depthAtLeast(depth, "tuefteln");
  if (text === null) {
    return (
      <Surface>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann zeigt Technik hier sein Textprojekt." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  const { status } = text;
  if (status === null) {
    return (
      <Surface>
        <p className={styles.note}>Wird gelesen …</p>
      </Surface>
    );
  }
  switch (status.state) {
    case "dspre":
      return <SetUp adoptable={status.adoptable} reason={status.reason} busy={text.busy} agentSetUp={text.agentSetUp === true} onSetUp={text.onSetUp} tinker={tinker} />;
    case "current":
      return <Current text={text} schema={status.schema} lastSyncAt={status.last_sync_at} tinker={tinker} />;
    case "older":
      return (
        <Surface lead={`Textprojekt, Schema-Version ${status.schema}: älter als diese Fassung von Gen4 Studio. Bis es angehoben ist, lässt es sich nur lesen.`}>
          {null}
        </Surface>
      );
    case "newer":
      return (
        <Surface>
          <Notice kind="error">Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version {status.schema}).</Notice>
        </Surface>
      );
    case "broken":
      return (
        <Surface>
          <Notice kind="error">{status.reason}</Notice>
        </Surface>
      );
  }
}

/** The surface: the title „Textprojekt“, below it `lead` and beside it `actions`, then what it holds. */
function Surface({ lead, actions, children }: { lead?: string; actions?: ReactNode; children?: ReactNode }) {
  const id = useId();
  const surface = useRef<HTMLElement>(null);
  const [, ask] = useState(0);
  useEffect(() => {
    const listener = () => ask((n) => n + 1);
    wantedListeners.add(listener);
    return () => void wantedListeners.delete(listener);
  }, []);
  // A request of a message's action: focus goes to its group, else to the title (a message closed by its action leaves focus on the body).
  useEffect(() => {
    const request = wanted;
    wanted = null;
    if (request === null || Date.now() - request.at > WANTED_FOR) return;
    const box = surface.current;
    (box?.querySelector<HTMLElement>(`[data-sync-${request.group}] button[aria-expanded]`) ?? box?.querySelector<HTMLElement>("h1"))?.focus();
  });
  return (
    <section ref={surface} className={styles.surface} data-technik="textprojekt" aria-labelledby={id}>
      <header className={styles.head}>
        <div className={styles.headText}>
          {/* tabIndex -1: where focus goes when the group a message leads to is not there. */}
          <h1 id={id} tabIndex={-1} className={styles.title}>
            Textprojekt
          </h1>
          {lead !== undefined && <p className={styles.text}>{lead}</p>}
        </div>
        {actions !== undefined && <div className={styles.actions}>{actions}</div>}
      </header>
      {children}
    </section>
  );
}

/**
 * A DSPRE folder: what a text project is, and „Als Textprojekt einrichten…“, locked with the reason when it cannot be set up, and while a
 * build or a set-up runs; an agent's set-up is said.
 */
function SetUp({
  adoptable,
  reason,
  busy,
  agentSetUp,
  onSetUp,
  tinker,
}: {
  adoptable: boolean;
  reason: string | null;
  busy: boolean;
  agentSetUp: boolean;
  onSetUp?: () => void;
  tinker: boolean;
}) {
  const reasonId = useId();
  const locked = !adoptable || busy || agentSetUp || onSetUp === undefined;
  const why = agentSetUp || adoptable ? null : reason;
  return (
    <Surface lead={SETUP_LEAD}>
      <div className={styles.actions}>
        <Button kind="primary" aria-disabled={locked || undefined} aria-describedby={why !== null ? reasonId : undefined} onClick={() => !locked && onSetUp?.()}>
          Als Textprojekt einrichten…
        </Button>
        {why !== null && (
          <p id={reasonId} className={styles.note}>
            {why}
          </p>
        )}
        {agentSetUp && (
          <p role="status" className={styles.note}>
            {AGENT_SETTING_UP}
          </p>
        )}
      </div>
      {/* The DSPRE layout has no tombstones, but a new entry at the end can have its number freed. */}
      {tinker && (
        <div className={styles.groups}>
          <DeleteSection />
        </div>
      )}
    </Surface>
  );
}

/** A text project of the current schema: its state, the sync under way, and what the last sync found. */
function Current({ text, schema, lastSyncAt, tinker }: { text: TextProjectView; schema: number; lastSyncAt: number | null; tinker: boolean }) {
  const now = useNow();
  const { sync } = text;
  const locked = sync.running || text.busy;
  // An agent's set-up still reported (the end follows its reopen): nothing is taken back meanwhile.
  const revertLocked = text.onRevert === undefined || text.busy || text.agentSetUp === true;
  const synced = lastSyncAt === null ? "Noch nicht abgeglichen." : `Zuletzt abgeglichen ${ago(lastSyncAt, now)}.`;
  const conflicts = sync.last?.conflicts ?? [];
  const broken = sync.last?.broken ?? [];
  const groups = useRef<HTMLDivElement>(null);
  // The conflict whose choice went through, and the order of the rows then: its row goes when the sync lists what is still open.
  const resolved = useRef<{ key: string; order: readonly string[] } | null>(null);
  const order = useRef<readonly string[]>([]);
  order.current = conflicts.map((c) => c.key);
  // When that row is gone and focus fell to the body with it, focus goes to the toggle of the next conflict, else the previous one, else
  // the group, else the title. Focus that is elsewhere by then (the user moved on) is left alone.
  useEffect(() => {
    const done = resolved.current;
    if (done === null || order.current.includes(done.key)) return;
    resolved.current = null;
    const active = document.activeElement;
    if (active !== null && active !== document.body) return;
    const box = groups.current;
    if (box === null) return;
    const toggle = (key: string) => box.querySelector<HTMLElement>(`[data-conflict="${CSS.escape(key)}"] button[aria-expanded]`);
    const at = done.order.indexOf(done.key);
    const neighbour = [...done.order.slice(at + 1), ...done.order.slice(0, at).reverse()].find((key) => toggle(key) !== null);
    const target = (neighbour === undefined ? null : toggle(neighbour)) ?? box.querySelector<HTMLElement>("[data-sync-conflicts] button[aria-expanded]") ?? box.closest("section")?.querySelector<HTMLElement>("h1");
    target?.focus();
  }, [sync.last]);
  const onChosen = (key: string) => {
    resolved.current = { key, order: order.current };
  };
  return (
    <Surface
      lead={`Textprojekt, Schema-Version ${schema}. ${synced}`}
      actions={
        <>
          <Button kind="secondary" aria-disabled={locked || undefined} onClick={() => !locked && void sync.run()}>
            Jetzt abgleichen
          </Button>
          <Button kind="secondary" aria-disabled={revertLocked || undefined} onClick={() => !revertLocked && text.onRevert?.()}>
            Zurücknehmen…
          </Button>
        </>
      }
    >
      {sync.running && (
        <div className={styles.progress}>
          <p role="status" className={styles.text}>
            {sync.progress === null ? "Gleicht ab …" : syncProgressText(sync.progress)}
          </p>
          {lastSyncAt === null && <p className={styles.note}>{FIRST_SYNC_NOTE}</p>}
        </div>
      )}
      {sync.error !== null && <Notice kind="error">{sync.error}</Notice>}
      <div ref={groups} className={styles.groups}>
        {conflicts.length > 0 && <Conflicts conflicts={conflicts} busy={text.busy} onChosen={onChosen} onResolved={text.onResolved} />}
        {broken.length > 0 && <Broken entries={broken} />}
        <Taken taken={sync.taken} />
        {tinker && <DeleteSection />}
      </div>
    </Surface>
  );
}

/** A section that is open until folded. */
function Group({ title, count, data, children }: { title: string; count?: number; data: string; children: ReactNode }) {
  const [open, setOpen] = useState(true);
  return (
    <div {...{ [data]: "" }}>
      <Section title={title} count={count} open={open} onToggle={() => setOpen(!open)}>
        {children}
      </Section>
    </div>
  );
}

/** „Auf zwei Seiten geändert“: a row per conflict, folded until asked for. */
function Conflicts({
  conflicts,
  busy,
  onChosen,
  onResolved,
}: {
  conflicts: readonly ConflictEntry[];
  busy: boolean;
  onChosen: (key: string) => void;
  onResolved: (assets: AssetId[]) => void;
}) {
  return (
    <Group title="Auf zwei Seiten geändert" count={conflicts.length} data="data-sync-conflicts">
      <ul className={styles.rows}>
        {conflicts.map((c) => (
          <ConflictRow
            key={c.key}
            conflict={c}
            busy={busy}
            onResolved={(assets) => {
              onChosen(c.key);
              onResolved(assets);
            }}
          />
        ))}
      </ul>
    </Group>
  );
}

/**
 * One conflict: its label and „Konflikt“ (or „Ungespeichert“ when the store holds the entry unsaved); unfolded the diff of both sides
 * and the choice, with a warning above it when taking the other side drops unsaved changes.
 */
function ConflictRow({ conflict, busy, onResolved }: { conflict: ConflictEntry; busy: boolean; onResolved: (assets: AssetId[]) => void }) {
  const [open, setOpen] = useState(false);
  const bodyId = useId();
  return (
    <li className={styles.row} data-conflict={conflict.key}>
      <div className={styles.rowHead}>
        <Button kind="quiet" size="s" icon={open ? "caretDown" : "caretRight"} aria-expanded={open} aria-controls={open ? bodyId : undefined} onClick={() => setOpen(!open)}>
          {conflict.label}
        </Button>
        {conflict.unsaved ? <Badge kind="unsaved" /> : <Badge kind="warning">Konflikt</Badge>}
      </div>
      {open && (
        <div id={bodyId} className={styles.rowBody}>
          <ConflictBody conflict={conflict} busy={busy} onResolved={onResolved} />
        </div>
      )}
    </li>
  );
}

type DiffState = { diff: ConflictDiff | null; error: string | null };

/** `busy`: a build runs, the choice waits (it would write a member the build is reading). */
function ConflictBody({ conflict, busy, onResolved }: { conflict: ConflictEntry; busy: boolean; onResolved: (assets: AssetId[]) => void }) {
  const { key, label, unsaved, other } = conflict;
  const [read, setRead] = useState<DiffState>({ diff: null, error: null });
  const [resolving, setResolving] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    conflictDiff(key).then(
      (diff) => current && setRead({ diff, error: null }),
      (e: unknown) => current && setRead({ diff: null, error: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }, [key]);
  const take = other === "files" ? "Geänderte Datei übernehmen" : "DSPRE-Fassung übernehmen";
  const { diff, error } = read;
  // The Studio's side of a unit held unsaved is its unsaved value (what the editor shows): the column says so.
  const columns: TableColumn[] = [
    { key: "path", label: "Feld" },
    { key: "studio", label: diff?.unsaved ? "Studio (ungespeichert)" : "Studio" },
    { key: "other", label: other === "files" ? "Datei" : "DSPRE" },
  ];
  const locked = resolving || busy;
  const choose = async (keep: Keep) => {
    if (locked) return;
    setResolving(true);
    setRefused(null);
    try {
      onResolved(await resolveExternalConflict(key, keep));
    } catch (e) {
      setRefused(errorText(e));
    } finally {
      setResolving(false);
    }
  };
  return (
    <>
      {diff === null && error === null && <p className={styles.note}>Lese die Abweichungen …</p>}
      {error !== null && <Notice kind="error">{error}</Notice>}
      {diff !== null && (
        <Table
          ariaLabel={`Abweichungen: ${label}`}
          columns={columns}
          rows={diff.lines.map((line, i) => ({
            key: String(i),
            cells: { path: <span className={styles.code}>{line.path}</span>, studio: <span className={styles.code}>{line.studio}</span>, other: <span className={styles.code}>{line.dspre}</span> },
          }))}
          footer={diff.more > 0 ? <span className={styles.note}>{moreText(diff.more)}</span> : undefined}
        />
      )}
      {unsaved && <Notice kind="warning">„{take}“ verwirft die ungespeicherten Änderungen an diesem Eintrag.</Notice>}
      <div className={styles.actions}>
        <Button kind="secondary" size="s" aria-disabled={locked || undefined} onClick={() => void choose("studio")}>
          Studio-Fassung behalten
        </Button>
        <Button kind="secondary" size="s" aria-disabled={locked || undefined} onClick={() => void choose("dspre")}>
          {take}
        </Button>
      </div>
      {refused !== null && <Notice kind="error">{refused}</Notice>}
    </>
  );
}

/** „Fehlerhafte Dateien“: a row per unit whose source does not read, each problem with its file, line and column. */
function Broken({ entries }: { entries: readonly BrokenEntry[] }) {
  return (
    <Group title="Fehlerhafte Dateien" count={entries.length} data="data-sync-broken">
      <ul className={styles.rows}>
        {entries.map((entry) => (
          <li key={entry.key} className={styles.row} data-broken={entry.key}>
            <span className={styles.label}>{entry.label}</span>
            <ul className={styles.problems}>
              {entry.problems.map((p, i) => (
                <li key={i}>
                  <code className={styles.code}>{problemLine(p)}</code>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </Group>
  );
}

/** „Zuletzt übernommen“: what the last takeover brought in, by side. */
function Taken({ taken }: { taken: SyncOutcome | null }) {
  const entries = taken === null ? [] : [...taken.adopted.map((e) => ({ entry: e, from: "aus DSPRE" })), ...taken.applied.map((e) => ({ entry: e, from: "aus Datei" }))];
  return (
    <Group title="Zuletzt übernommen" count={entries.length > 0 ? entries.length : undefined} data="data-sync-taken">
      {entries.length === 0 ? (
        <p className={`${styles.text} ${styles.inset}`}>Nichts von außen geändert.</p>
      ) : (
        <ul className={styles.rows}>
          {entries.map(({ entry, from }) => (
            <li key={`${from}\u0000${entry.key}`} className={`${styles.row} ${styles.rowHead}`} data-taken={entry.key}>
              <span className={styles.label}>{entry.label}</span>
              <Badge kind="neutral">{from}</Badge>
            </li>
          ))}
        </ul>
      )}
    </Group>
  );
}

/**
 * The kinds „Löschen“ of Tüfteln offers: the entities no head deletes, and the map (its head's entry is locked by the cell it shows in;
 * here it can be deleted once no cell holds it).
 */
type TinkerDeleteKind = "matrix" | "map" | "event_file" | "area_data" | "script" | "text_archive";

const DELETE_KINDS: readonly SelectOption<TinkerDeleteKind>[] = [
  { value: "matrix", label: "Matrix" },
  { value: "map", label: "Karte" },
  { value: "event_file", label: "Eventdatei" },
  { value: "area_data", label: "Gebietsdaten" },
  { value: "script", label: "Scriptdatei" },
  { value: "text_archive", label: "Textarchiv" },
];

/** The entity of `kind` with the number `n` (a script file names its file). */
const deleteTarget = (kind: TinkerDeleteKind, n: number): DeleteTarget => (kind === "script" ? { kind, file: n } : { kind, index: n });

/** „Löschen“ (Tüfteln): a kind and a number, and „Löschen…“ opens the dialog for that entity (the dialog says what is possible). */
function DeleteSection() {
  const remove = useDelete();
  const [kind, setKind] = useState<TinkerDeleteKind>("matrix");
  const [number, setNumber] = useState<number | null>(null);
  return (
    <Group title="Löschen" data="data-delete-section">
      <div className={`${styles.actions} ${styles.deleteRow} ${styles.inset}`}>
        <SelectField label="Art" showLabel value={kind} options={DELETE_KINDS} onChange={setKind} />
        <NumberField label="Nummer" showLabel value={number} onCommit={setNumber} min={0} bits={16} />
        <Button kind="secondary" aria-disabled={number === null || undefined} onClick={() => number !== null && remove.open(deleteTarget(kind, number))}>
          Löschen…
        </Button>
      </div>
    </Group>
  );
}
