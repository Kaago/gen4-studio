import { useId, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { AssetId, ChangedEntry, ChangeState, OriginalStatus } from "../api";
import { useReset } from "../original/OriginalContext";
import { RESET_LABEL } from "../original/resetEntry";
import { Badge, Button, EmptyState, ListRow, Notice, Section, useRovingList } from "../ui/primitives";
import type { BadgeKind } from "../ui/primitives";
import styles from "./ChangesSurface.module.css";
import type { ChangesApi } from "./useChanges";

const STATE_BADGE: Record<ChangeState, { kind: BadgeKind; text: string }> = {
  changed: { kind: "warning", text: "Geändert" },
  new: { kind: "info", text: "Neu" },
  missing: { kind: "error", text: "Fehlt" },
};

/** Why the row's reset button is locked; null when it can be used. */
function locked(entry: ChangedEntry): string | null {
  if (entry.state !== "changed") return "Kein Original";
  if (entry.target === null) return "Für diese Art gibt es kein Zurücksetzen.";
  return null;
}

/** The line under „Änderungen“ in the list of Technik; `changes` null: no project. */
export function changesSubtitle(changes: ChangesApi | null, status: OriginalStatus | null): string {
  if (changes === null) return "Kein Projekt geöffnet";
  if (status === null) return "Wird gelesen …";
  if (status.state === "unavailable") return "Basis-ROM fehlt";
  if (changes.busy) return "Vergleiche …";
  if (changes.report === null) return changes.error === null ? "Noch nicht verglichen" : "Vergleich fehlgeschlagen";
  const n = changes.report.entries.length;
  if (n === 0) return "Keine Abweichung";
  return n === 1 ? "1 Eintrag weicht ab" : `${n} Einträge weichen ab`;
}

/** The entries grouped, the groups in the order of their first entry (the kernel sorts them). */
function groupsOf(entries: readonly ChangedEntry[]): [string, ChangedEntry[]][] {
  const groups = new Map<string, ChangedEntry[]>();
  for (const e of entries) {
    const list = groups.get(e.group);
    if (list === undefined) groups.set(e.group, [e]);
    else list.push(e);
  }
  return [...groups];
}

const sameAsset = (a: AssetId, b: AssetId) => a.kind === b.kind && a.index === b.index;

export type ChangesSurfaceProps = {
  /** The compare of the open project (`useChanges`, held by the app); null without a project. */
  changes: ChangesApi | null;
  /** The base ROM as resetting needs it; null while it is read. */
  status: OriginalStatus | null;
  /** The store's assets with unsaved edits: their rows say „Ungespeichert“ (the list counts the saved state). */
  dirty: readonly AssetId[];
  onOpenProject: () => void;
};

/**
 * Technik › Änderungen (4.1b): what of the saved project differs from the base ROM, one section per group, one row per entity with its
 * state and „Auf Original zurücksetzen…“ (the app's dialog, through `useReset`). Without a base ROM it names the reason and where the base
 * is assigned. The compare itself runs in `useChanges`; this surface only shows it and offers „Neu vergleichen“.
 */
export function ChangesSurface({ changes, status, dirty, onOpenProject }: ChangesSurfaceProps) {
  if (changes === null) {
    return (
      <Surface>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann zeigt Technik hier, was vom Original abweicht." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  if (status === null) {
    return (
      <Surface>
        <p className={styles.note}>Wird gelesen …</p>
      </Surface>
    );
  }
  if (status.state === "unavailable") {
    return (
      <Surface>
        <Notice kind="warning">{status.reason}</Notice>
        <p className={styles.text}>Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.</p>
      </Surface>
    );
  }
  const { report, progress, error, busy } = changes;
  return (
    <Surface
      lead={`Gegenüber der Basis-ROM ${status.label}. Gezählt wird der gespeicherte Stand.`}
      action={
        <Button kind="secondary" aria-disabled={busy || undefined} onClick={() => !busy && void changes.compare()}>
          Neu vergleichen
        </Button>
      }
    >
      {busy && (
        <div className={styles.progress}>
          <p role="status" className={styles.text}>
            {progress === null ? "Vergleiche …" : `Vergleiche ${progress.done} von ${progress.total} Dateien …`}
          </p>
          {report === null && <p className={styles.note}>Das erste Mal dauert länger; danach geht es schnell.</p>}
        </div>
      )}
      {error !== null && <Notice kind="error">{error}</Notice>}
      {report !== null && report.unbuilt.length > 0 && (
        <Notice kind="info">Noch nicht gebaut: {report.unbuilt.join(", ")}. Nach dem nächsten Build geht der Vergleich schneller.</Notice>
      )}
      {report !== null &&
        (report.entries.length === 0 ? (
          <p className={styles.text}>Alles wie in der Basis-ROM.</p>
        ) : (
          <ChangeGroups entries={report.entries} dirty={dirty} />
        ))}
    </Surface>
  );
}

/** The surface: the title „Änderungen“, below it `lead` and beside it `action`, then what it holds. */
function Surface({ lead, action, children }: { lead?: string; action?: ReactNode; children: ReactNode }) {
  const id = useId();
  return (
    <section className={styles.surface} data-technik="aenderungen" aria-labelledby={id}>
      <header className={styles.head}>
        <div className={styles.headText}>
          <h1 id={id} className={styles.title}>
            Änderungen
          </h1>
          {lead !== undefined && <p className={styles.text}>{lead}</p>}
        </div>
        {action}
      </header>
      {children}
    </section>
  );
}

/** The groups of the report, each a section (open until folded) holding a list of its rows. */
function ChangeGroups({ entries, dirty }: { entries: readonly ChangedEntry[]; dirty: readonly AssetId[] }) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const [selected, setSelected] = useState<string | null>(null);
  const toggle = (group: string) =>
    setFolded((before) => {
      const next = new Set(before);
      if (!next.delete(group)) next.add(group);
      return next;
    });
  return (
    <div className={styles.groups}>
      {groupsOf(entries).map(([group, list]) => (
        <div key={group} data-changes-group={group}>
          <Section title={group} count={list.length} open={!folded.has(group)} onToggle={() => toggle(group)}>
            <GroupList group={group} entries={list} dirty={dirty} selected={selected} onSelect={setSelected} />
          </Section>
        </div>
      ))}
    </div>
  );
}

function GroupList({ group, entries, dirty, selected, onSelect }: { group: string; entries: readonly ChangedEntry[]; dirty: readonly AssetId[]; selected: string | null; onSelect: (key: string) => void }) {
  const { open } = useReset();
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  return (
    <div ref={list} role="listbox" aria-label={group} className={styles.list}>
      {entries.map((entry) => {
        const key = `${group}\u0000${entry.label}`;
        const badge = STATE_BADGE[entry.state];
        const unsaved = entry.assets.some((a) => dirty.some((d) => sameAsset(a, d)));
        const why = locked(entry);
        const target = entry.target;
        return (
          <ListRow
            key={key}
            label={entry.label}
            ariaLabel={[entry.label, badge.text, ...(unsaved ? ["Ungespeichert"] : [])].join(", ")}
            selected={selected === key}
            onSelect={() => onSelect(key)}
            dataAttrs={{ "data-change": entry.label }}
            trailing={
              <span className={styles.trailing}>
                <Badge kind={badge.kind}>{badge.text}</Badge>
                {unsaved && <Badge kind="unsaved" />}
                <Button kind="secondary" size="s" disabled={why !== null} title={why ?? undefined} onClick={() => target !== null && why === null && open(target)}>
                  {RESET_LABEL}
                </Button>
              </span>
            }
          />
        );
      })}
    </div>
  );
}
