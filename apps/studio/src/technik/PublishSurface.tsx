import { useEffect, useId, useRef, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { formatBytes } from "../api";
import type { BaseCheck, BaseStatus } from "../api";
import { openPath } from "../platform/dialogs";
import { Badge, Button, Dialog, EmptyState, Notice } from "../ui/primitives";
import type { BadgeKind } from "../ui/primitives";
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
  found: "Bereit für einen Patch",
};

/** The line under „Veröffentlichen“ in the list of Technik; null: no project. */
export function publishSubtitle(base: BaseApi | null): string {
  if (base === null) return "Kein Projekt geöffnet";
  if (base.status === null) return base.error === null ? "Wird gelesen …" : "Nicht lesbar";
  if (base.status.state === "found" && base.lastPatch !== null) return "Patch erstellt";
  return SUBTITLE[base.status.state];
}

const PICK = { directory: false, title: "Basis-ROM wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] };

/** The last part of a path, `/` or `\` separated. */
function fileName(path: string): string {
  return path.split(/[/\\]/).filter((part) => part !== "").pop() ?? path;
}

/** Why there is no patch yet; null when there can be one. */
function patchBlocked(status: BaseStatus | null): string | null {
  if (status === null || status.state === "found") return null;
  if (status.state === "unassigned") return "Erst eine Basis-ROM zuordnen.";
  return status.reason;
}

/**
 * Technik › Veröffentlichen: the card „Basis-ROM“ (state, assign, search a moved ROM) and the card „Patch“ (create it, its
 * result). Assigning asks first with what the check found; the patch asks first too. Nothing here opens by itself: the
 * suggestion of the `.nds` next to the folder shows only in this card and in the list's subtitle. While something runs here, or a
 * build or save of the app (`building`), no action that changes the project is offered. `base` is null without a project.
 *
 * Busy blocks the buttons with `aria-disabled` and a guard in the click, not with `disabled`, so the button with keyboard focus keeps
 * it while its step runs. Focus then goes on by one rule (`next`): „Patch erstellen…“ when it is enabled (the next step), else the
 * button that started the step. A dialog returns it there when it closes.
 */
export function PublishSurface({ base, building, onOpenProject }: { base: BaseApi | null; building: boolean; onOpenProject: () => void }) {
  if (base === null) {
    return (
      <Surface>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann erstellt Technik hier seinen Patch." action={{ label: "Projekt öffnen", onClick: onOpenProject }} />
      </Surface>
    );
  }
  return <Publish base={base} building={building} />;
}

function Surface({ children }: { children: ReactNode }) {
  const id = useId();
  return (
    <section className={styles.surface} data-technik="veroeffentlichen" aria-labelledby={id}>
      <header className={styles.head}>
        <h1 id={id} className={styles.title}>
          Veröffentlichen
        </h1>
      </header>
      {children}
    </section>
  );
}

/** A `.nds` checked and waiting for the answer of the dialog „Basis-ROM zuordnen?“. */
type Asking = { nds: string; check: BaseCheck };

function Publish({ base, building }: { base: BaseApi; building: boolean }) {
  const { status, busy } = base;
  const [asking, setAsking] = useState<Asking | null>(null);
  const [askingPatch, setAskingPatch] = useState(false);
  // „Basis-ROM zugeordnet“ shows after an assignment until the next step starts.
  const [assigned, setAssigned] = useState(false);
  // The button that started the step in the base card, and „Patch erstellen…“; `settled` asks for focus once the step has ended.
  const opener = useRef<HTMLElement | null>(null);
  const patchButton = useRef<HTMLButtonElement>(null);
  const [settled, setSettled] = useState(false);
  const baseHeading = useId();
  const patchHeading = useId();
  const noteId = useId();
  const running = busy !== null || building;

  // Where focus goes when a step of the base card ends: the next step if it can start, else the button of this one if it is still there.
  const next = () => {
    const patch = patchButton.current;
    if (patch !== null && !patch.disabled) return patch;
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
  const createPatch = () => {
    setAskingPatch(false);
    setAssigned(false);
    void base.createPatch();
  };

  // A failure shows in the card whose button started the step (reading the status belongs to „Basis-ROM“).
  const patchError = base.errorFrom === "patch" ? base.error : null;
  const baseError = base.errorFrom === "patch" ? null : base.error;
  const blocked = patchBlocked(status);
  // The note beside „Patch erstellen…“: why it waits, or that it runs.
  const patchNote = busy === "patch" ? "Baue und erstelle den Patch …" : blocked;
  const plaque = status === null ? null : PLAQUE[status.state];
  const name = status !== null && status.state === "found" ? status.patch_name : null;
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

      <section className={styles.card} aria-labelledby={patchHeading} data-patch-card="">
        <div className={styles.cardHead}>
          <h2 id={patchHeading} className={styles.heading}>
            Patch
          </h2>
        </div>
        <p className={styles.text}>Der Patch enthält nur deine Änderungen. Wer dieselbe Basis-ROM hat, wendet ihn mit xdelta3 oder Rom Patcher JS an.</p>
        <div className={styles.actions}>
          <Button
            ref={patchButton}
            kind="primary"
            disabled={status?.state !== "found"}
            aria-disabled={running || undefined}
            aria-describedby={patchNote !== null ? noteId : undefined}
            onClick={begin(() => setAskingPatch(true))}
          >
            Patch erstellen…
          </Button>
          {patchNote !== null && (
            <span id={noteId} className={styles.note}>
              {patchNote}
            </span>
          )}
        </div>
        {base.lastPatch !== null && busy !== "patch" && patchError === null && (
          <Notice kind="success" title={`Patch erstellt: ${formatBytes(base.lastPatch.size)}`}>
            <span className={styles.result}>
              <span className={styles.path} title={base.lastPatch.patch}>
                {base.lastPatch.patch}
              </span>
              <span>Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.</span>
            </span>
          </Notice>
        )}
        {patchError !== null && <Notice kind="error">{patchError}</Notice>}
      </section>

      <AssignDialog asking={asking} assigning={busy === "assign"} onClose={() => setAsking(null)} onAssign={(nds) => void assign(nds)} finalFocus={next} />
      <Dialog
        open={askingPatch}
        onOpenChange={setAskingPatch}
        title="Patch erstellen?"
        finalFocus={() => patchButton.current}
        description={`Speichert, baut die ROM und schreibt build/${name ?? ""}.xdelta und build/${name ?? ""}.xdelta.txt. Ein älterer Patch dort wird ersetzt.`}
        actions={[
          { label: "Abbrechen", onClick: () => setAskingPatch(false) },
          { label: "Patch erstellen", kind: "primary", onClick: createPatch },
        ]}
      />
    </Surface>
  );
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
