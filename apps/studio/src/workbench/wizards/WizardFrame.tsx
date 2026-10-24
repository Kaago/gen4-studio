import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { Badge, Button, Icon, Notice } from "../../ui/primitives";
import type { IconName } from "../../ui/primitives";
import { escapeCloses } from "../modal";
import { WizardBar } from "./WizardBar";
import { wizardStatus } from "./wizardStatus";
import styles from "./WizardFrame.module.css";

export interface WizardStep {
  id: string;
  title: string;
  content: ReactNode;
  /** Whether "Weiter" (on the last step: the submit button) is enabled. */
  canNext: boolean;
}

export type WizardFrameProps = {
  title: string;
  /** The symbol of the bar's chip. */
  icon: IconName;
  steps: WizardStep[];
  step: number;
  onStep: (index: number) => void;
  /** The work is running: nothing can be clicked, no key acts. */
  busy: boolean;
  /** A refusal of the last submit (plain backend text; nothing was written): it takes the summary's place, the wizard stays where it is. */
  error?: string | null;
  /** What the wizard is going to do, in one sentence: a line below every step while it is set. */
  summary?: string;
  /** What it does cannot be undone: the summary line carries the badge "Nicht rückgängig". */
  irreversible?: boolean;
  /** What the wizard did; when set, it replaces the step, with "Zeigen", "Hier spielen" and "Schließen" below. */
  result?: ReactNode | null;
  submitLabel: string;
  onSubmit: () => void;
  /** "Abbrechen", "Schließen" and Escape. */
  onCancel: () => void;
  /** "Zeigen": shows what the wizard made (only for a result that has something to show). */
  onShow?: () => void;
  /** "Hier spielen" beside "Zeigen". */
  onPlay?: () => void;
};

/** Where Enter belongs to the control itself (it presses, picks or breaks the line) and never moves the wizard on. */
const OWN_ENTER = "button, a, textarea, [role=option], [role=combobox], [role=radio], [role=tab]";
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

type Slots = { bar: HTMLElement; hand: HTMLElement };

/** The wizard's places in the deck: the slot in the world's hinge and the one beside its hand dock (none: rendered where it stands). */
function findSlots(): Slots | null {
  const bar = document.querySelector<HTMLElement>("[data-wizard-bar]");
  const hand = document.querySelector<HTMLElement>("[data-wizard-hand]");
  return bar && hand ? { bar, hand } : null;
}

const resting = () => document.activeElement === null || document.activeElement === document.body;

/**
 * The frame of a wizard in the deck: its step bar goes into the hinge, the step itself into the hand (in place of the hand's dock), both
 * through portals into the slots the deck keeps for them; without them (a unit test) both render where the frame stands. The step is a
 * dialog named by the title in the bar, not modal: the view above stays usable.
 *
 * Keys, from anywhere in the step or the bar: Enter moves on (or submits on the last step) unless a control used it (a field that commits
 * its typed text, a button, a list) or it lands in a text area; Escape cancels by the rules of `escapeCloses` (a field takes back its
 * typed text first); Cmd+[ goes back. Opening puts the focus on the step's first control, closing gives it back to where it was.
 */
export function WizardFrame({ title, icon, steps, step, onStep, busy, error = null, summary, irreversible = false, result = null, submitLabel, onSubmit, onCancel, onShow, onPlay }: WizardFrameProps) {
  const titleId = useId();
  const frame = useRef<HTMLDivElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  const [slots, setSlots] = useState(findSlots);
  // What had the focus before the wizard opened: it gets it back when the wizard goes.
  const [before] = useState(() => (document.activeElement instanceof HTMLElement ? document.activeElement : null));

  const current = steps[step];
  const done = result !== null;
  const last = step === steps.length - 1;

  const latest = useRef({ step, busy, done, last, canNext: current?.canNext ?? false, onStep, onSubmit, onCancel });
  latest.current = { step, busy, done, last, canNext: current?.canNext ?? false, onStep, onSubmit, onCancel };

  // The deck's slots: when the wizard opens together with the deck that holds them, they exist only after this commit.
  useLayoutEffect(() => {
    const found = findSlots();
    if (found?.bar !== slots?.bar || found?.hand !== slots?.hand) setSlots(found);
  }, []);

  useEffect(() => {
    const inside = (target: EventTarget | null): target is Element => target instanceof Element && (frame.current?.contains(target) === true || bar.current?.contains(target) === true);
    const advance = () => {
      const s = latest.current;
      if (!s.canNext) return;
      if (s.last) s.onSubmit();
      else s.onStep(s.step + 1);
    };
    const onKey = (e: KeyboardEvent) => {
      const s = latest.current;
      if (e.key === "Enter") {
        if (!inside(e.target) || e.metaKey || e.ctrlKey || e.altKey || e.shiftKey || e.isComposing) return;
        if (e.defaultPrevented) {
          // A field committed its typed text: it keeps the focus (or gets it back), so the next Enter moves on from there.
          if (e.target instanceof HTMLElement && e.target.matches("[data-draft]") && e.target.isConnected && resting()) e.target.focus();
          return;
        }
        // A held key repeats: it must not run through the steps into the submit that cannot be undone.
        if (s.busy || s.done || e.repeat || e.target.closest(OWN_ENTER) !== null) return;
        e.preventDefault();
        advance();
      } else if (e.key === "Escape") {
        if (s.busy || !inside(e.target) || !escapeCloses(e)) return;
        e.preventDefault();
        s.onCancel();
      } else if (e.key === "[" && (e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey) {
        // In a browser this would go back a page.
        e.preventDefault();
        if (!s.busy && !s.done && s.step > 0) s.onStep(s.step - 1);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      if (before?.isConnected) before.focus();
    };
  }, []);

  // The first control of the step (or of the result) takes the focus when the wizard opens, and when the focus fell out of the wizard because
  // what had it went away (the step that took an Enter, the button that started the work) or was locked while the work ran (an inert frame
  // and disabled buttons drop the focus; the work may end on the same step: the patch applied, a refused submit, a failed "Zeigen").
  const focusFirst = () => {
    const box = frame.current;
    if (!box) return;
    (box.querySelector<HTMLElement>(FOCUSABLE) ?? box).focus();
  };
  useEffect(focusFirst, [slots]);
  useEffect(() => {
    if (!busy && resting()) focusFirst();
  }, [step, done, busy]);

  useEffect(() => {
    wizardStatus.set({ step, count: steps.length, title: current?.title ?? "", done });
  }, [step, steps.length, current?.title, done]);
  useEffect(() => () => wizardStatus.set(null), []);

  const stepBar = (
    <div ref={bar} className={styles.barHost}>
      <WizardBar
        titleId={titleId}
        title={title}
        icon={icon}
        steps={steps.map((s) => s.title)}
        step={step}
        done={done}
        busy={busy}
        canNext={current?.canNext ?? false}
        submitLabel={submitLabel}
        onCancel={onCancel}
        onBack={() => onStep(step - 1)}
        onNext={() => (last ? onSubmit() : onStep(step + 1))}
      />
    </div>
  );

  let foot: ReactNode = null;
  if (!done && error) {
    foot = (
      <div className={styles.foot}>
        <Notice kind="error">{error}</Notice>
      </div>
    );
  } else if (!done && summary) {
    foot = (
      <div className={`${styles.foot} ${styles.summary}`}>
        <Icon name="list" />
        <span className={styles.sentence}>{summary}</span>
        {irreversible && <Badge kind="irreversible" />}
      </div>
    );
  }

  const body = (
    <div ref={frame} className={styles.frame} role="dialog" aria-labelledby={titleId} data-wizard="" tabIndex={-1} inert={busy} aria-busy={busy || undefined}>
      {done ? (
        <div className={styles.result}>
          <div className={styles.content}>
            <div>{result}</div>
          </div>
          <div className={styles.resultActions}>
            {onShow && (
              <Button kind="primary" onClick={onShow}>
                Zeigen
              </Button>
            )}
            {onShow && onPlay && (
              <Button kind="play" icon="play" onClick={onPlay}>
                Hier spielen
              </Button>
            )}
            <Button kind="secondary" onClick={onCancel}>
              Schließen
            </Button>
          </div>
        </div>
      ) : (
        <div className={styles.content}>{current?.content}</div>
      )}
      {foot}
    </div>
  );

  if (slots) {
    return (
      <>
        {createPortal(stepBar, slots.bar)}
        {createPortal(body, slots.hand)}
      </>
    );
  }
  return (
    <>
      {stepBar}
      {body}
    </>
  );
}
