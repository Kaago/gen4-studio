import { useId, useLayoutEffect, useRef, useState } from "react";
import type { FocusEvent, ReactNode, Ref } from "react";
import type { AssetId, ChangedEntry, ChangeState, OriginalStatus } from "../api";
import { useReset } from "../original/OriginalContext";
import { RESET_LABEL } from "../original/resetEntry";
import { Badge, Button, EmptyState, ListRow, Notice, Section, Tooltip, useRovingList } from "../ui/primitives";
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

/** At most this many folders are named in „Noch nicht gebaut“; the others are counted. */
const UNBUILT_NAMED = 5;

/** The folders without a build entry: up to five by name, more as „und {n} weitere“ (a project never built has dozens). */
export function unbuiltText(folders: readonly string[]): string {
  const named = folders.slice(0, UNBUILT_NAMED).join(", ");
  const more = folders.length - UNBUILT_NAMED;
  return more > 0 ? `${named} und ${more} weitere` : named;
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

/** The key of a row: its group and label (what `GroupList` keys its rows by). */
const rowKey = (group: string, label: string) => `${group}\u0000${label}`;

/** The part of a row that holds focus: the row itself or its reset button; `el` is the element that had it. */
type Held = { key: string; part: "row" | "button"; el: HTMLElement };

/** The rows shown under `surface`, by key, in the order of the DOM (the dialog's own `data-change` lines live in its portal, not here). */
function shownRows(surface: HTMLElement): Map<string, HTMLElement> {
  const rows = new Map<string, HTMLElement>();
  for (const row of surface.querySelectorAll<HTMLElement>("[data-change]")) {
    const group = row.closest<HTMLElement>("[data-changes-group]")?.dataset.changesGroup ?? "";
    rows.set(rowKey(group, row.dataset.change ?? ""), row);
  }
  return rows;
}

/** The element of `row` that is `part`: its reset button (the only button a row holds) or the row itself. */
const partOf = (row: HTMLElement | undefined, part: Held["part"]) => (part === "button" ? row?.querySelector<HTMLElement>("button") : row) ?? null;

/** What `useKeepFocus` gives the surface and what it offers the rows. */
type KeepFocus = {
  ref: Ref<HTMLElement>;
  onFocus: (e: FocusEvent<HTMLElement>) => void;
  /**
   * For the `part` of row `key`, shown now: a function that says where focus belongs when asked later: that element; if the row is gone by
   * then, the same part of its neighbour (by the order of the rows now); else „Neu vergleichen“.
   */
  where: (key: string, part: Held["part"]) => () => HTMLElement | null;
};

/**
 * Keeps keyboard focus on the same place of the list while it changes under it: when the row that holds it disappears (a reset made it
 * equal to the base again), focus goes to the same part of the next row that is still there (by the order before), else the previous one,
 * else „Neu vergleichen“, else the title. When the row stays but its element is replaced by a new one (a remount), focus goes to the new
 * one. Without it the removed element drops focus to the body. The row is remembered across a phase without any rows (the surface shows no
 * list while it is read or compared): focus waits on „Neu vergleichen“ (the title) meanwhile, so it never rests on the body. Focus that is
 * elsewhere by then (a dialog, another control) is left alone; the reset dialog asks `where` when it closes. Spread the result on the surface.
 */
function useKeepFocus(): KeepFocus {
  const ref = useRef<HTMLElement>(null);
  // The row whose part had focus last, the order of the rows when the list was last shown, and the element focus waits on while no row is shown.
  const held = useRef<Held | null>(null);
  const before = useRef<string[]>([]);
  const parked = useRef<HTMLElement | null>(null);
  // Our own `focus()` calls: moving focus to a row holds that row, but waiting on „Neu vergleichen“ is no choice of the user (the row stays held).
  const own = useRef(false);
  const focus = (el: HTMLElement | null | undefined) => {
    own.current = true;
    el?.focus();
    own.current = false;
  };
  const fallbackOf = (surface: HTMLElement) => surface.querySelector<HTMLElement>("[data-compare-again]") ?? surface.querySelector<HTMLElement>("h1");
  /** Where the `part` of row `key` is among `rows`, `order` being the order the rows had when the list was last whole. */
  const resolve = (surface: HTMLElement, rows: Map<string, HTMLElement>, order: readonly string[], key: string, part: Held["part"]) => {
    const same = rows.get(key);
    if (same !== undefined) return partOf(same, part);
    const i = order.indexOf(key);
    const neighbour = i < 0 ? undefined : (order.slice(i + 1).find((k) => rows.has(k)) ?? order.slice(0, i).reverse().find((k) => rows.has(k)));
    return partOf(neighbour === undefined ? undefined : rows.get(neighbour), part) ?? fallbackOf(surface);
  };
  const where = (key: string, part: Held["part"]) => {
    const order = before.current;
    return () => (ref.current === null ? null : resolve(ref.current, shownRows(ref.current), order, key, part));
  };
  const onFocus = (e: FocusEvent<HTMLElement>) => {
    const target = e.target;
    const row = target.closest<HTMLElement>("[data-change]");
    const group = row?.closest<HTMLElement>("[data-changes-group]");
    if (own.current && row == null) return;
    parked.current = null;
    held.current = row == null || group == null ? null : { key: rowKey(group.dataset.changesGroup ?? "", row.dataset.change ?? ""), part: target.closest("button") !== null ? "button" : "row", el: target };
  };
  // After every commit: the rows are read from the DOM, as shown.
  useLayoutEffect(() => {
    const surface = ref.current;
    if (surface === null) return;
    const rows = shownRows(surface);
    const was = before.current;
    if (rows.size > 0) before.current = [...rows.keys()];
    const lost = held.current;
    if (lost === null) return;
    const now = document.activeElement;
    const free = now === null || now === document.body;
    if (rows.has(lost.key)) {
      parked.current = null;
      // The row is there; if the element that held focus was replaced, focus follows to the new one.
      if (!lost.el.isConnected && free) focus(partOf(rows.get(lost.key), lost.part));
      return;
    }
    const mine = free || (parked.current !== null && now === parked.current);
    if (rows.size === 0) {
      // No rows shown (yet): wait on the fallback, and keep the row for the list that follows.
      const fallback = fallbackOf(surface);
      if (mine && fallback !== null) {
        focus(fallback);
        parked.current = fallback;
      }
      return;
    }
    held.current = null;
    parked.current = null;
    if (mine) focus(resolve(surface, rows, was, lost.key, lost.part));
  });
  return { ref, onFocus, where };
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
  const { where, ...keep } = useKeepFocus();
  if (changes === null) {
    return (
      <Surface {...keep}>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann zeigt Technik hier, was vom Original abweicht." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  if (status === null) {
    return (
      <Surface {...keep}>
        <p className={styles.note}>Wird gelesen …</p>
      </Surface>
    );
  }
  if (status.state === "unavailable") {
    return (
      <Surface {...keep}>
        <Notice kind="warning">{status.reason}</Notice>
        <p className={styles.text}>Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.</p>
      </Surface>
    );
  }
  const { report, progress, error, busy } = changes;
  return (
    <Surface
      {...keep}
      lead={`Gegenüber der Basis-ROM ${status.label}. Gezählt wird der gespeicherte Stand.`}
      action={
        <Button kind="secondary" data-compare-again="" aria-disabled={busy || undefined} onClick={() => !busy && void changes.compare()}>
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
        <Notice kind="info">Noch nicht gebaut: {unbuiltText(report.unbuilt)}. Nach dem nächsten Build geht der Vergleich schneller.</Notice>
      )}
      {report !== null &&
        (report.entries.length === 0 ? (
          <p className={styles.text}>Alles wie in der Basis-ROM.</p>
        ) : (
          <ChangeGroups entries={report.entries} dirty={dirty} where={where} />
        ))}
    </Surface>
  );
}

/** The surface: the title „Änderungen“, below it `lead` and beside it `action`, then what it holds. */
function Surface({ lead, action, children, ref, onFocus }: { lead?: string; action?: ReactNode; children: ReactNode; ref?: Ref<HTMLElement>; onFocus?: (e: FocusEvent<HTMLElement>) => void }) {
  const id = useId();
  return (
    <section ref={ref} onFocus={onFocus} className={styles.surface} data-technik="aenderungen" aria-labelledby={id}>
      <header className={styles.head}>
        <div className={styles.headText}>
          {/* tabIndex -1: the last place focus can go when the list is empty and „Neu vergleichen“ is gone. */}
          <h1 id={id} tabIndex={-1} className={styles.title}>
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
function ChangeGroups({ entries, dirty, where }: { entries: readonly ChangedEntry[]; dirty: readonly AssetId[]; where: KeepFocus["where"] }) {
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
            <GroupList group={group} entries={list} dirty={dirty} where={where} selected={selected} onSelect={setSelected} />
          </Section>
        </div>
      ))}
    </div>
  );
}

function GroupList({ group, entries, dirty, where, selected, onSelect }: { group: string; entries: readonly ChangedEntry[]; dirty: readonly AssetId[]; where: KeepFocus["where"]; selected: string | null; onSelect: (key: string) => void }) {
  const { open } = useReset();
  const list = useRef<HTMLDivElement>(null);
  const noteBase = useId();
  useRovingList(list);
  return (
    <div ref={list} role="listbox" aria-label={group} className={styles.list}>
      {entries.map((entry) => {
        const key = rowKey(group, entry.label);
        const badge = STATE_BADGE[entry.state];
        const unsaved = entry.assets.some((a) => dirty.some((d) => sameAsset(a, d)));
        const why = locked(entry);
        const target = entry.target;
        const noteId = `${noteBase}${entries.indexOf(entry)}`;
        // A locked button stays focusable (aria-disabled): the reason is its description and the tooltip, which keyboard focus opens too.
        const button = (
          <Button
            kind="secondary"
            size="s"
            aria-disabled={why !== null || undefined}
            aria-describedby={why !== null ? noteId : undefined}
            // The dialog asks where focus goes when it closes: this button, or its neighbour once the reset has taken the row away.
            onClick={() => target !== null && why === null && open(target, { finalFocus: where(key, "button") })}
          >
            {RESET_LABEL}
          </Button>
        );
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
                {why === null ? (
                  button
                ) : (
                  <>
                    <Tooltip content={why}>{button}</Tooltip>
                    <span id={noteId} hidden>
                      {why}
                    </span>
                  </>
                )}
              </span>
            }
          />
        );
      })}
    </div>
  );
}
