import { useEffect, useId, useRef, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { formatBytes } from "../api";
import type { BaseCheck, BaseStatus, ReleaseEntry, ReleasesDto } from "../api";
import { openPath } from "../platform/dialogs";
import { dateText, publishLocked } from "../publish/publishWords";
import { Badge, Button, Dialog, EmptyState, IconButton, Notice, Table } from "../ui/primitives";
import type { BadgeKind, TableColumn } from "../ui/primitives";
import styles from "./PublishSurface.module.css";
import type { BaseApi } from "./useBase";

type BaseState = BaseStatus["state"];

const PLAQUE: Record<BaseState, { kind: BadgeKind; text: string }> = {
  found: { kind: "success", text: "Gefunden" },
  missing: { kind: "warning", text: "Fehlt" },
  unassigned: { kind: "neutral", text: "Nicht zugeordnet" },
  unreadable: { kind: "error", text: "Nicht lesbar" },
};

const SUBTITLE: Record<BaseState, string> = {
  unassigned: "Keine Basis-ROM zugeordnet",
  missing: "Basis-ROM fehlt",
  unreadable: "Basismarke nicht lesbar",
  found: "Bereit zum Veröffentlichen",
};

/**
 * The line under „Veröffentlichen“ in the list of Technik: the base's state while it is not found (it locks publishing first, as
 * `publishLocked` says), then „Nicht lesbar“ for a broken list of releases, „Veröffentlicht v0.2.0“ after the last release, else
 * „Bereit zum Veröffentlichen“.
 */
export function publishSubtitle(base: BaseApi | null): string {
  if (base === null) return "Kein Projekt geöffnet";
  if (base.status === null) return base.error === null ? "Wird gelesen …" : "Nicht lesbar";
  if (base.status.state !== "found") return SUBTITLE[base.status.state];
  if (base.releasesError !== null) return "Nicht lesbar";
  const list = base.releases?.releases ?? [];
  const last = list[list.length - 1];
  return last !== undefined ? `Veröffentlicht v${last.version}` : SUBTITLE.found;
}

const PICK = { directory: false, title: "Basis-ROM wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] };

/** The last part of a path, `/` or `\` separated. */
function fileName(path: string): string {
  return path.split(/[/\\]/).filter((part) => part !== "").pop() ?? path;
}

/** A path that does not hang from the project root: `/…`, `\\…` or a drive (`C:\…`). */
const ABSOLUTE = /^(?:[/\\]|[A-Za-z]:[/\\])/;

/** The full path of a release file: as the list has it when absolute, else from the project root (the parent of `dir`, `<root>/build`). */
function releasePath(list: ReleasesDto, file: string): string {
  if (ABSOLUTE.test(file)) return file;
  const root = list.dir.slice(0, Math.max(list.dir.lastIndexOf("/"), list.dir.lastIndexOf("\\")));
  return `${root}/${file}`;
}

type PublishSurfaceProps = {
  base: BaseApi | null;
  /** A build or save of the app runs. */
  building: boolean;
  onOpenProject: () => void;
  /** „Veröffentlichen…“: the app opens the wizard. */
  onPublish: () => void;
  /** „Im Finder zeigen“ of a release file (the path as the list has it). */
  onReveal: (path: string) => Promise<void>;
};

/**
 * Technik › Veröffentlichen: the head (what a patch is), the card „Basis-ROM“ (state, assign, search a moved ROM) and the card
 * „Veröffentlichen“ (the wizard's button and the list „Bisherige Veröffentlichungen“). Assigning asks first with what the check found.
 * Nothing here opens by itself: the suggestion of the `.nds` next to the folder shows only in this card and in the list's subtitle.
 * While something runs here, or a build or save of the app (`building`), no action that changes the project is offered. `base` is null
 * without a project.
 *
 * Busy blocks the buttons with `aria-disabled` and a guard in the click, not with `disabled`, so the button with keyboard focus keeps
 * it while its step runs. Focus then goes on by one rule (`next`): „Veröffentlichen…“ when it is enabled (the next step), else the
 * button that started the step. A dialog returns it there when it closes.
 */
export function PublishSurface({ base, building, onOpenProject, onPublish, onReveal }: PublishSurfaceProps) {
  if (base === null) {
    return (
      <Surface>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann veröffentlichst du es hier." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  return <Publish base={base} building={building} onPublish={onPublish} onReveal={onReveal} />;
}

function Surface({ children }: { children: ReactNode }) {
  const id = useId();
  return (
    <section className={styles.surface} data-technik="veroeffentlichen" aria-labelledby={id}>
      <header className={styles.head}>
        <h1 id={id} className={styles.title}>
          Veröffentlichen
        </h1>
        <p className={styles.lead}>Ein Patch gibt deine Änderungen weiter, ohne die ROM: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.</p>
      </header>
      {children}
    </section>
  );
}

/** A `.nds` checked and waiting for the answer of the dialog „Basis-ROM zuordnen?“. */
type Asking = { nds: string; check: BaseCheck };

function Publish({ base, building, onPublish, onReveal }: { base: BaseApi; building: boolean; onPublish: () => void; onReveal: (path: string) => Promise<void> }) {
  const { status, busy } = base;
  const [asking, setAsking] = useState<Asking | null>(null);
  // „Basis-ROM zugeordnet“ shows after an assignment until the next step starts.
  const [assigned, setAssigned] = useState(false);
  // The button that started the step in the base card, and „Veröffentlichen…“; `settled` asks for focus once the step has ended.
  const opener = useRef<HTMLElement | null>(null);
  const publishButton = useRef<HTMLButtonElement>(null);
  const [settled, setSettled] = useState(false);
  const baseHeading = useId();
  const publishHeading = useId();
  const noteId = useId();
  const running = busy !== null || building;

  // Where focus goes when a step of the base card ends: the next step if it can start, else the button of this one if it is still there.
  const next = () => {
    const publish = publishButton.current;
    if (publish !== null && !publish.disabled) return publish;
    return opener.current?.isConnected ? opener.current : null;
  };
  // A step that opens no dialog ends with its button gone (found) or with focus taken by the native file dialog: it goes to `next`,
  // unless the user has put it somewhere else meanwhile.
  useEffect(() => {
    if (!settled || running) return;
    setSettled(false);
    const active = document.activeElement;
    if (active === null || active === document.body) next()?.focus();
  }, [settled, running, status]);
  /** The click of a button that starts a step: nothing while something runs, and the button is remembered. */
  const begin = (start: () => void) => (e: MouseEvent<HTMLButtonElement>) => {
    if (running) return;
    opener.current = e.currentTarget;
    start();
  };

  const ask = async (nds: string) => {
    setAssigned(false);
    const check = await base.check(nds);
    if (check !== null) setAsking({ nds, check });
    else setSettled(true);
  };
  const pickAndAsk = async () => {
    const nds = await openPath(PICK);
    if (nds !== null) await ask(nds);
    else setSettled(true);
  };
  // The dialog stays open with what it assigns until the assignment is through, then closes.
  const assign = async (nds: string) => {
    await base.assign(nds);
    setAsking(null);
    setAssigned(true);
  };
  const search = async () => {
    setAssigned(false);
    const path = await openPath(PICK);
    if (path !== null) await base.register(path);
    setSettled(true);
  };
  // The wizard is the next step: the note of the assignment goes.
  const publish = () => {
    if (running) return;
    setAssigned(false);
    onPublish();
  };

  // Every step here belongs to „Basis-ROM“ (reading the status too); the release itself fails in the wizard.
  const baseError = base.error;
  // „Veröffentlichen…“ needs a found base ROM and a readable list; the note beside it says why it waits.
  const canPublish = status?.state === "found" && base.releasesError === null;
  const lock = publishLocked(status, base.releasesError);
  const plaque = status === null ? null : PLAQUE[status.state];
  // After an assignment that went through: the status names the ROM.
  const assignedLabel = assigned && baseError === null && status !== null && (status.state === "found" || status.state === "missing") ? status.label : null;

  return (
    <Surface>
      <section className={styles.card} aria-labelledby={baseHeading} data-base-state={status?.state ?? "loading"}>
        <div className={styles.cardHead}>
          <h2 id={baseHeading} className={styles.heading}>
            Basis-ROM
          </h2>
          {plaque && <Badge kind={plaque.kind}>{plaque.text}</Badge>}
        </div>
        <p className={styles.text}>Ein Patch beschreibt den Unterschied zu deiner Basis-ROM. Die ROM bleibt außerhalb des Projekts; Gen4 Studio merkt sich nur, wo sie liegt.</p>
        {status === null && baseError === null && <p className={styles.note}>Wird gelesen …</p>}
        {status?.state === "found" && (
          <dl className={styles.facts}>
            <div className={styles.fact}>
              <dt>ROM</dt>
              <dd>{status.label}</dd>
            </div>
            <div className={styles.fact}>
              <dt>SHA-1</dt>
              <dd className={styles.mono} title={status.mark.sha1}>
                {status.mark.sha1.slice(0, 8)}
              </dd>
            </div>
            <div className={`${styles.fact} ${styles.wide}`}>
              <dt>Datei</dt>
              <dd className={styles.path} title={status.path}>
                {status.path}
              </dd>
            </div>
          </dl>
        )}
        {status?.state === "unassigned" &&
          (status.suggestion !== null ? (
            <>
              <Notice kind="info">Neben dem Projekt liegt „{fileName(status.suggestion)}“.</Notice>
              <div className={styles.actions}>
                <Button kind="primary" aria-disabled={running || undefined} onClick={begin(() => void ask(status.suggestion!))}>
                  Zuordnen…
                </Button>
                <Button kind="secondary" aria-disabled={running || undefined} onClick={begin(() => void pickAndAsk())}>
                  Andere Datei wählen…
                </Button>
              </div>
            </>
          ) : (
            <>
              <p className={styles.text}>Diesem Projekt ist keine Basis-ROM zugeordnet.</p>
              <div className={styles.actions}>
                <Button kind="primary" aria-disabled={running || undefined} onClick={begin(() => void pickAndAsk())}>
                  Basis-ROM zuordnen…
                </Button>
              </div>
            </>
          ))}
        {status?.state === "missing" && (
          <>
            <Notice kind="warning">{status.reason}</Notice>
            <div className={styles.actions}>
              <Button kind="primary" aria-disabled={running || undefined} onClick={begin(() => void search())}>
                Basis-ROM suchen…
              </Button>
            </div>
          </>
        )}
        {status?.state === "unreadable" && <Notice kind="error">{status.reason}</Notice>}
        {assignedLabel !== null && busy === null && <Notice kind="success">Basis-ROM zugeordnet: {assignedLabel}.</Notice>}
        {baseError !== null && <Notice kind="error">{baseError}</Notice>}
      </section>

      <section className={styles.card} aria-labelledby={publishHeading} data-publish-card="">
        <div className={styles.cardHead}>
          <h2 id={publishHeading} className={styles.heading}>
            Veröffentlichen
          </h2>
        </div>
        <p className={styles.text}>Der Assistent fragt Version und Änderungsnotizen, baut die ROM und schreibt den Patch mit Begleitdatei an den Ort, den du wählst.</p>
        <div className={styles.actions}>
          <Button
            ref={publishButton}
            kind="primary"
            disabled={!canPublish}
            aria-disabled={running || undefined}
            aria-describedby={lock !== null ? noteId : undefined}
            onClick={publish}
          >
            Veröffentlichen…
          </Button>
          {lock !== null && (
            <span id={noteId} className={styles.note}>
              {lock}
            </span>
          )}
        </div>
        <h3 className={styles.subheading}>Bisherige Veröffentlichungen</h3>
        {base.releasesError !== null ? (
          <Notice kind="error">{base.releasesError}</Notice>
        ) : base.releases === null ? (
          <p className={styles.note}>Wird gelesen …</p>
        ) : base.releases.releases.length === 0 ? (
          <p className={styles.note}>Noch keine Veröffentlichung.</p>
        ) : (
          <Releases list={base.releases} onReveal={onReveal} />
        )}
      </section>

      <AssignDialog asking={asking} assigning={busy === "assign"} onClose={() => setAsking(null)} onAssign={(nds) => void assign(nds)} finalFocus={next} />
    </Surface>
  );
}

const RELEASE_COLUMNS: readonly TableColumn[] = [
  { key: "version", label: "Version", width: 88 },
  { key: "date", label: "Datum", width: 96 },
  { key: "file", label: "Datei" },
  { key: "size", label: "Größe", width: 88, align: "end" },
  { key: "notes", label: "Notizen" },
  { key: "reveal", label: "Aktion", width: 44, hideLabel: true },
];

/** A release's row: the file's name (its full path as the title), the notes' first line (the rest as the title) and „Im Finder zeigen“. */
function releaseRow(list: ReleasesDto, entry: ReleaseEntry, onReveal: (path: string) => Promise<void>) {
  const [first = "", ...rest] = entry.notes.split(/\r?\n/u);
  const more = rest.join("\n").trim();
  return {
    key: entry.version,
    attrs: { "data-release": entry.version },
    cells: {
      version: entry.version,
      date: dateText(entry.date),
      file: (
        <span className={styles.path} title={releasePath(list, entry.file)}>
          {fileName(entry.file)}
        </span>
      ),
      size: formatBytes(entry.size),
      notes: more === "" ? first : <span title={more}>{first}</span>,
      // Each row's button is named by its version, so a screen reader tells them apart.
      reveal: <IconButton label={`Im Finder zeigen: v${entry.version}`} icon="open" onClick={() => void onReveal(entry.file)} />,
    },
  };
}

/** „Bisherige Veröffentlichungen“, newest first. */
function Releases({ list, onReveal }: { list: ReleasesDto; onReveal: (path: string) => Promise<void> }) {
  const rows = [...list.releases].reverse().map((entry) => releaseRow(list, entry, onReveal));
  return <Table columns={RELEASE_COLUMNS} rows={rows} ariaLabel="Bisherige Veröffentlichungen" />;
}

/**
 * „Basis-ROM zuordnen?“ with what the check found: the warnings, or the refusal (then only „Schließen“). While `assigning`
 * it keeps its content, its buttons are disabled and it does not close.
 */
function AssignDialog({
  asking,
  assigning,
  onClose,
  onAssign,
  finalFocus,
}: {
  asking: Asking | null;
  assigning: boolean;
  onClose: () => void;
  onAssign: (nds: string) => void;
  finalFocus: () => HTMLElement | null;
}) {
  const info = asking?.check.info;
  const refused = asking?.check.refused ?? null;
  return (
    <Dialog
      open={asking !== null}
      onOpenChange={(open) => !open && !assigning && onClose()}
      title="Basis-ROM zuordnen?"
      finalFocus={finalFocus}
      description={info ? `${info.title} · ${info.game_code} · SHA-1 ${info.sha1.slice(0, 8)}` : undefined}
      actions={
        refused !== null
          ? [{ label: "Schließen", onClick: onClose }]
          : [
              { label: "Abbrechen", onClick: onClose, disabled: assigning },
              { label: "Zuordnen", kind: "primary", onClick: () => asking && onAssign(asking.nds), disabled: assigning },
            ]
      }
    >
      {asking && (
        <div className={styles.dialogBody}>
          {info?.known_base && <p className={styles.text}>Bekannte Basis-ROM: {info.known_base}</p>}
          {refused !== null ? (
            <Notice kind="error">{refused}</Notice>
          ) : (
            <>
              {asking.check.warnings.map((w, i) => (
                <Notice key={`${w.kind}-${i}`} kind="warning">
                  {w.message}
                </Notice>
              ))}
              <div className={styles.actions}>
                <span className={styles.text}>Schreibt die Basismarke .g4/base.json ins Projekt.</span>
                <Badge kind="irreversible" />
              </div>
            </>
          )}
        </div>
      )}
    </Dialog>
  );
}
