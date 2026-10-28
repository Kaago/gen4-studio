import { useEffect, useId, useState } from "react";
import type { ReactNode } from "react";
import { conflictDiff, errorText, resolveExternalConflict } from "../api";
import type { AssetId, BrokenEntry, ConflictDiff, ConflictEntry, Keep, SyncOutcome, TextProjectStatus } from "../api";
import type { ExternalSyncApi } from "../app/useExternalSync";
import { ago, useNow } from "../ui/frame/timeAgo";
import { Badge, Button, EmptyState, Notice, Section, Table } from "../ui/primitives";
import type { TableColumn } from "../ui/primitives";
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

export type TextProjectSurfaceProps = {
  /** The text project of the open project; null without a project. */
  text: TextProjectView | null;
  onOpenProject: () => void;
};

/**
 * Technik › Textprojekt (4.3b): the folder's state as a text project. A DSPRE folder offers „Als Textprojekt einrichten…“ (locked with the
 * reason when it cannot be set up); a text project names its schema and its last sync, offers „Jetzt abgleichen“ and „Zurücknehmen…“, and
 * lists what the sync found: the conflicts with their diff and a choice each („Auf zwei Seiten geändert“), the files that do not read
 * („Fehlerhafte Dateien“) and what the last takeover brought („Zuletzt übernommen“).
 */
export function TextProjectSurface({ text, onOpenProject }: TextProjectSurfaceProps) {
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
      return <SetUp adoptable={status.adoptable} reason={status.reason} busy={text.busy} agentSetUp={text.agentSetUp === true} onSetUp={text.onSetUp} />;
    case "current":
      return <Current text={text} schema={status.schema} lastSyncAt={status.last_sync_at} />;
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
  return (
    <section className={styles.surface} data-technik="textprojekt" aria-labelledby={id}>
      <header className={styles.head}>
        <div className={styles.headText}>
          <h1 id={id} className={styles.title}>
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
function SetUp({ adoptable, reason, busy, agentSetUp, onSetUp }: { adoptable: boolean; reason: string | null; busy: boolean; agentSetUp: boolean; onSetUp?: () => void }) {
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
    </Surface>
  );
}

/** A text project of the current schema: its state, the sync under way, and what the last sync found. */
function Current({ text, schema, lastSyncAt }: { text: TextProjectView; schema: number; lastSyncAt: number | null }) {
  const now = useNow();
  const { sync } = text;
  const locked = sync.running || text.busy;
  // An agent's set-up still reported (the end follows its reopen): nothing is taken back meanwhile.
  const revertLocked = text.onRevert === undefined || text.busy || text.agentSetUp === true;
  const synced = lastSyncAt === null ? "Noch nicht abgeglichen." : `Zuletzt abgeglichen ${ago(lastSyncAt, now)}.`;
  const conflicts = sync.last?.conflicts ?? [];
  const broken = sync.last?.broken ?? [];
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
      <div className={styles.groups}>
        {conflicts.length > 0 && <Conflicts conflicts={conflicts} busy={text.busy} onResolved={text.onResolved} />}
        {broken.length > 0 && <Broken entries={broken} />}
        <Taken taken={sync.taken} />
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
function Conflicts({ conflicts, busy, onResolved }: { conflicts: readonly ConflictEntry[]; busy: boolean; onResolved: (assets: AssetId[]) => void }) {
  return (
    <Group title="Auf zwei Seiten geändert" count={conflicts.length} data="data-sync-conflicts">
      <ul className={styles.rows}>
        {conflicts.map((c) => (
          <ConflictRow key={c.key} conflict={c} busy={busy} onResolved={onResolved} />
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
