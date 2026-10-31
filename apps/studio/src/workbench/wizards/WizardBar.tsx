import { Button, Icon, Kbd } from "../../ui/primitives";
import type { IconName } from "../../ui/primitives";
import styles from "./WizardBar.module.css";

export type WizardBarProps = {
  /** The id of the title: the wizard's dialog is named by it. */
  titleId: string;
  title: string;
  icon: IconName;
  /** The titles of the steps, in order. */
  steps: readonly string[];
  step: number;
  /** The result shows: every step is ticked and the buttons go (the result has its own). */
  done: boolean;
  busy: boolean;
  /** Whether "Weiter" (on the last step: the submit button) can be pressed. */
  canNext: boolean;
  submitLabel: string;
  /** The step whose button submits (default the last; a wizard whose result is a pill of its own submits on the step before it). */
  submitAt?: number;
  onCancel: () => void;
  onBack: () => void;
  onNext: () => void;
};

type StepState = "done" | "current" | "open";

/**
 * The step bar of a wizard in the hinge: the area symbol and the title on the left, the numbered steps as pills in the middle (a tick for
 * the ones behind), "Abbrechen", "Zurück" (from the second step on) and "Weiter" or the submit button on the right. The keys stand beside
 * their buttons, so a button says only what it does.
 */
export function WizardBar({ titleId, title, icon, steps, step, done, busy, canNext, submitLabel, submitAt = steps.length - 1, onCancel, onBack, onNext }: WizardBarProps) {
  const last = step === submitAt;
  const stateOf = (i: number): StepState => (done || i < step ? "done" : i === step ? "current" : "open");
  return (
    <div className={styles.bar} inert={busy}>
      <div className={styles.head}>
        <span className={styles.chip} aria-hidden="true">
          <Icon name={icon} weight="fill" />
        </span>
        <h2 id={titleId} className={styles.title}>
          {title}
        </h2>
      </div>
      <ol className={styles.steps} aria-label="Schritte">
        {steps.map((name, i) => {
          const state = stateOf(i);
          return (
            <li key={`${i}-${name}`} className={styles.step} data-state={state} aria-current={state === "current" ? "step" : undefined} aria-label={`${i + 1}. ${name}`}>
              {i > 0 && <span className={styles.link} aria-hidden="true" />}
              <span className={styles.pill}>
                <span className={styles.mark} aria-hidden="true">
                  {state === "done" ? <Icon name="check" /> : i + 1}
                </span>
                <span className={styles.name}>{name}</span>
              </span>
            </li>
          );
        })}
      </ol>
      {!done && (
        <div className={styles.actions}>
          <Button kind="quiet" disabled={busy} onClick={onCancel}>
            Abbrechen
          </Button>
          {step > 0 && (
            <span className={styles.keyed}>
              <Button kind="secondary" disabled={busy} onClick={onBack} aria-keyshortcuts="Meta+BracketLeft">
                Zurück
              </Button>
              <Kbd keys="⌘[" onHinge />
            </span>
          )}
          <span className={styles.keyed}>
            <Button kind="primary" disabled={busy || !canNext} onClick={onNext} aria-keyshortcuts="Enter">
              {last ? submitLabel : "Weiter"}
            </Button>
            <Kbd keys="↵" onHinge />
          </span>
        </div>
      )}
    </div>
  );
}
