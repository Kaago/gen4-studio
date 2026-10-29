import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { FocusEvent, ReactNode, Ref } from "react";
import type { Against, AssetId, ChangedEntry, DiffSubject, GitStatus, OriginalStatus } from "../api";
import type { GitStatusApi } from "../git/useGitStatus";
import { useReset } from "../original/OriginalContext";
import { RESET_LABEL } from "../original/resetEntry";
import { Badge, Button, EmptyState, ListRow, Notice, Section, Segmented, Tooltip, useRovingList } from "../ui/primitives";
import type { SegmentedOption } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import styles from "./ChangesSurface.module.css";
import { rowResetLock, stateBadge } from "./diffWords";
import { EntityDiff } from "./EntityDiff";
import type { ChangesApi } from "./useChanges";

/** The three comparisons of the switch above the list. */
const AGAINST: readonly SegmentedOption<Against>[] = [
  { value: "base", label: "Gegen Basis" },
  { value: "head", label: "Gegen letzten Commit" },
  { value: "saved", label: "Gegen gespeichert" },
];


/** At most this many folders are named in „Noch nicht gebaut“; the others are counted. */
const UNBUILT_NAMED = 5;

/** The folders without a build entry: up to five by name, more as „und {n} weitere“ (a project never built has dozens). */
export function unbuiltText(folders: readonly string[]): string {
  const named = folders.slice(0, UNBUILT_NAMED).join(", ");
  const more = folders.length - UNBUILT_NAMED;
  return more > 0 ? `${named} und ${more} weitere` : named;
}

/**
 * The line under „Änderungen“ in the list of Technik, in the words of the comparison; `changes` null: no project. Against the last commit
 * and the saved state the entities are counted (the status line counts paths).
 */
export function changesSubtitle(against: Against, changes: ChangesApi | null, status: OriginalStatus | null, git: GitStatus | null, unsaved: readonly ChangedEntry[] | null): string {
  if (changes === null) return "Kein Projekt geöffnet";
  if (against === "head") {
    if (git === null) return "Wird gelesen …";
    if (git.repo.kind === "none") return "Kein Git";
    if (git.head === null) return "Noch kein Commit";
    const n = git.entries.length;
    if (n === 0) return "Keine Änderung seit dem Commit";
    return n === 1 ? "1 Änderung seit dem Commit" : `${n} Änderungen seit dem Commit`;
  }
  if (against === "saved") {
    if (unsaved === null) return "Wird gelesen …";
    const n = unsaved.length;
    if (n === 0) return "Nichts ungespeichert";
    return n === 1 ? "1 Eintrag ungespeichert" : `${n} Einträge ungespeichert`;
  }
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

/** Whether two subjects name the same entity (the same in all three lists). */
function sameSubject(a: DiffSubject, b: DiffSubject): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "file") return b.kind === "file" && a.path === b.path;
  if (a.kind === "member") return b.kind === "member" && a.folder === b.folder && a.index === b.index;
  return b.kind !== "file" && b.kind !== "member" && a.index === b.index;
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
  /** Where focus goes when nothing of the list is left: „Neu vergleichen“, else the title. */
  fallback: () => HTMLElement | null;
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
  return { ref, onFocus, where, fallback: () => (ref.current === null ? null : fallbackOf(ref.current)) };
}

const sameAsset = (a: AssetId, b: AssetId) => a.kind === b.kind && a.index === b.index;

export type ChangesSurfaceProps = {
  /** The compare of the open project (`useChanges`, held by the app); null without a project. */
  changes: ChangesApi | null;
  /** The base ROM as resetting needs it; null while it is read. */
  status: OriginalStatus | null;
  /** The store's assets with unsaved edits: their rows say „Ungespeichert“ (the lists against the base and the commit count the saved state). */
  dirty: readonly AssetId[];
  onOpenProject: () => void;
  /** What is compared against; the app keeps the choice for the session. */
  against: Against;
  onAgainst: (against: Against) => void;
  /** The Git state of the project (the list against the last commit); null without a project. */
  git: GitStatusApi | null;
  /** The unsaved edits as entities (the list against the saved state); null while they are read. */
  unsaved: readonly ChangedEntry[] | null;
  /** Why the last read of the unsaved edits failed; null when it went through. */
  unsavedError?: string | null;
  /** Reads the unsaved edits again („Neu vergleichen“ after a failed read). */
  onReloadUnsaved?: () => void;
  /** „Git einrichten…“: the app's dialog. */
  onGitSetup: () => void;
  /** „Öffnen“ in the detail. */
  goTo: (target: NavTarget) => void;
  /** Moves whenever the compared state may differ: the detail reads its diff again. */
  revision: number;
  /**
   * The chosen entity, when the app holds it (so it survives „Öffnen“ and the way back to Technik); without it the surface holds its own.
   * It stays chosen in another comparison that lists it; one whose list lacks it forgets it (`onChoose(null)`).
   */
  chosen?: DiffSubject | null;
  onChoose?: (subject: DiffSubject | null) => void;
};

/** What a comparison shows: the sentence under the title, the button beside it, what comes before the list, the list and its empty text. */
type Shown = { lead?: string; action?: ReactNode; before?: ReactNode; entries: readonly ChangedEntry[] | null; empty: string };

/** The sentence about the last commit: „Gegenüber dem letzten Commit 4c1d2e3 „…“ auf main.“, detached „(losgelöst)“, in a larger repository its folder first. */
function headLead(git: GitStatus): string {
  const head = git.head!;
  const where = git.detached || git.branch === null ? " (losgelöst)" : ` auf ${git.branch}`;
  const repo = git.repo.kind === "enclosing" ? `Repository ${git.repo.workdir}. ` : "";
  return `${repo}Gegenüber dem letzten Commit ${head.short} „${head.summary}“${where}. Gezählt wird der gespeicherte Stand.`;
}

/**
 * Technik › Änderungen (4.1b, 4.4): what of the project differs from the base ROM, the last commit or the saved state (the switch above
 * the list), one section per group, one row per entity with its state and „Auf Original zurücksetzen…“ (the app's dialog, through
 * `useReset`); the chosen row's diff beside the list (below it on a narrow surface). Without a base ROM „Gegen Basis“ names the reason and
 * where the base is assigned; without a repository „Gegen letzten Commit“ offers „Git einrichten…“. The compares run in the app's hooks;
 * this surface only shows them.
 */
export function ChangesSurface(props: ChangesSurfaceProps) {
  const { changes, status, dirty, onOpenProject, against, onAgainst, git, unsaved, unsavedError = null, onReloadUnsaved = () => {}, onGitSetup, goTo, revision } = props;
  const { where, fallback, ...keep } = useKeepFocus();
  // The chosen entity (by its subject, the same in every comparison): the app's when it holds it, else this surface's.
  const [own, setOwn] = useState<DiffSubject | null>(null);
  const chosen = props.chosen !== undefined ? props.chosen : own;
  const choose = props.onChoose ?? setOwn;
  const shown = changes === null ? null : against === "head" ? headShown(git, onGitSetup) : against === "saved" ? savedShown(unsaved, unsavedError, onReloadUnsaved) : baseShown(changes, status);
  const entries = shown?.entries ?? null;
  const entry = chosen === null ? null : (entries?.find((e) => sameSubject(e.subject, chosen)) ?? null);
  // A list that is read and lacks the chosen entity forgets it (a list still read keeps it for when it comes).
  const lost = chosen !== null && entries !== null && entry === null;
  useEffect(() => {
    if (lost) choose(null);
  }, [lost, choose]);
  if (changes === null || shown === null) {
    return (
      <Surface {...keep}>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann zeigt Technik hier, was vom Original abweicht." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  const selected = entry === null ? null : rowKey(entry.group, entry.label);
  return (
    <Surface
      {...keep}
      lead={shown.lead}
      action={shown.action}
      toggle={
        <div className={styles.toggle} data-against={against}>
          <Segmented size="s" ariaLabel="Vergleichen mit" options={AGAINST} value={against} onChange={onAgainst} />
        </div>
      }
    >
      {shown.before}
      {entries !== null &&
        (entries.length === 0 ? (
          <p className={styles.text}>{shown.empty}</p>
        ) : (
          <div className={styles.split} data-changes-split="">
            <div className={styles.columns}>
              <ChangeGroups entries={entries} dirty={against === "saved" ? [] : dirty} against={against} status={status} where={where} selected={selected} onSelect={(key) => choose(entries.find((e) => rowKey(e.group, e.label) === key)?.subject ?? null)} />
              <EntityDiff entry={entry} against={against} revision={revision} headId={git?.status?.head?.id ?? null} goTo={goTo} finalFocus={fallback} />
            </div>
          </div>
        ))}
    </Surface>
  );
}

/** „Gegen Basis“ (4.1b): the compare against the base ROM, its progress, and why it cannot run. */
function baseShown(changes: ChangesApi, status: OriginalStatus | null): Shown {
  const empty = "Alles wie in der Basis-ROM.";
  if (status === null) return { before: <p className={styles.note}>Wird gelesen …</p>, entries: null, empty };
  if (status.state === "unavailable") {
    return {
      before: (
        <>
          <Notice kind="warning">{status.reason}</Notice>
          <p className={styles.text}>Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.</p>
        </>
      ),
      entries: null,
      empty,
    };
  }
  const { report, progress, error, busy } = changes;
  return {
    lead: `Gegenüber der Basis-ROM ${status.label}. Gezählt wird der gespeicherte Stand.`,
    action: (
      <Button kind="secondary" data-compare-again="" aria-disabled={busy || undefined} onClick={() => !busy && void changes.compare()}>
        Neu vergleichen
      </Button>
    ),
    before: (
      <>
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
      </>
    ),
    entries: report?.entries ?? null,
    empty,
  };
}

/** „Gegen letzten Commit“: the entries of the Git status; without a repository the offer to set one up, without a commit why nothing is compared. */
function headShown(git: GitStatusApi | null, onGitSetup: () => void): Shown {
  const empty = "Seit dem letzten Commit nichts geändert.";
  const status = git?.status ?? null;
  const failed = git?.error ?? null;
  if (status === null) {
    return { before: failed === null ? <p className={styles.note}>Wird gelesen …</p> : <Notice kind="error">{failed}</Notice>, entries: null, empty };
  }
  if (status.repo.kind === "none") {
    return {
      before: (
        <>
          <Notice kind="info">Dieses Projekt liegt in keinem Git-Repository.</Notice>
          <p className={styles.text}>Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.</p>
          <div className={styles.offer} data-git-offer="">
            <Button kind="secondary" onClick={onGitSetup}>
              Git einrichten…
            </Button>
          </div>
        </>
      ),
      entries: null,
      empty,
    };
  }
  if (status.head === null) return { before: <Notice kind="info">Noch kein Commit: es gibt nichts zu vergleichen.</Notice>, entries: null, empty };
  return {
    lead: headLead(status),
    action: (
      <Button kind="secondary" data-compare-again="" onClick={() => void git!.refresh()}>
        Neu vergleichen
      </Button>
    ),
    before: failed !== null && <Notice kind="error">{failed}</Notice>,
    entries: status.entries,
    empty,
  };
}

/** „Gegen gespeichert“: the store's unsaved edits as entities; a failed read names its reason and offers to read again. */
function savedShown(unsaved: readonly ChangedEntry[] | null, error: string | null, onReload: () => void): Shown {
  const empty = "Nichts ungespeichert.";
  const lead = "Was ungespeichert vom gespeicherten Stand abweicht.";
  if (error !== null) {
    return {
      lead,
      action: (
        <Button kind="secondary" data-compare-again="" onClick={onReload}>
          Neu vergleichen
        </Button>
      ),
      before: <Notice kind="error">{error}</Notice>,
      entries: unsaved,
      empty,
    };
  }
  if (unsaved === null) return { before: <p className={styles.note}>Wird gelesen …</p>, entries: null, empty };
  return { lead, entries: unsaved, empty };
}

/** The surface: the title „Änderungen“, below it `lead` and beside it `action`, then the switch of the comparison and what it holds. */
function Surface({ lead, action, toggle, children, ref, onFocus }: { lead?: string; action?: ReactNode; toggle?: ReactNode; children: ReactNode; ref?: Ref<HTMLElement>; onFocus?: (e: FocusEvent<HTMLElement>) => void }) {
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
      {toggle}
      {children}
    </section>
  );
}

type GroupsProps = {
  entries: readonly ChangedEntry[];
  dirty: readonly AssetId[];
  against: Against;
  status: OriginalStatus | null;
  where: KeepFocus["where"];
  selected: string | null;
  onSelect: (key: string) => void;
};

/** The groups of the list, each a section (open until folded) holding a list of its rows. */
function ChangeGroups({ entries, dirty, against, status, where, selected, onSelect }: GroupsProps) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
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
            <GroupList group={group} entries={list} dirty={dirty} against={against} status={status} where={where} selected={selected} onSelect={onSelect} />
          </Section>
        </div>
      ))}
    </div>
  );
}

function GroupList({ group, entries, dirty, against, status, where, selected, onSelect }: Omit<GroupsProps, "entries"> & { group: string; entries: readonly ChangedEntry[] }) {
  const { open } = useReset();
  const list = useRef<HTMLDivElement>(null);
  const noteBase = useId();
  useRovingList(list);
  return (
    <div ref={list} role="listbox" aria-label={group} className={styles.list}>
      {entries.map((entry) => {
        const key = rowKey(group, entry.label);
        const badge = stateBadge(against, entry.state);
        const unsaved = entry.assets.some((a) => dirty.some((d) => sameAsset(a, d)));
        const why = rowResetLock(entry, status);
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
