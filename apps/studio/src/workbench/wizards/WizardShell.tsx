import { useEffect, useRef } from "react";
import type { CSSProperties, ReactNode } from "react";
import { escapeCloses, useFocusTrap } from "../modal";

export interface WizardStep {
  id: string;
  title: string;
  content: ReactNode;
  /** Whether "Weiter" (on the last step: the submit button) is enabled. */
  canNext: boolean;
}

interface Props {
  title: string;
  steps: WizardStep[];
  step: number;
  onStep: (index: number) => void;
  submitLabel: string;
  onSubmit: () => void;
  onClose: () => void;
  /** The work is running: nothing can be clicked and Escape does not close. */
  busy: boolean;
  /** What the wizard did; when set, it replaces the steps and only "Schließen" is left. */
  result: ReactNode | null;
  /** A refusal of the last "Anlegen" (plain backend text; nothing was written): shown beside the buttons, the wizard stays where it is. */
  error?: string | null;
}

const backdrop: CSSProperties = { position: "fixed", inset: 0, background: "rgba(0,0,0,0.45)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 50 };
const sheet: CSSProperties = {
  background: "#fff",
  borderRadius: 8,
  width: 820,
  maxWidth: "94vw",
  maxHeight: "90vh",
  display: "flex",
  flexDirection: "column",
  fontFamily: "system-ui, sans-serif",
  fontSize: 13,
  color: "#111827",
  outline: "none",
};
const footer: CSSProperties = { display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 8, padding: "10px 16px", borderTop: "1px solid #e5e7eb" };
const banner: CSSProperties = { flex: 1, color: "#b91c1c", overflowWrap: "anywhere", maxHeight: 80, overflowY: "auto" };

/** The modal frame of a wizard: the step list on the left, one step's content on the right, back / next / submit below. */
export function WizardShell({ title, steps, step, onStep, submitLabel, onSubmit, onClose, busy, result, error = null }: Props) {
  const dialog = useRef<HTMLDivElement>(null);
  const close = useRef(onClose);
  close.current = onClose;
  const working = useRef(busy);
  working.current = busy;

  useFocusTrap(dialog);
  useEffect(() => {
    dialog.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      // Escape that takes back typed text, or clears a search, does not also close the wizard (see `escapeCloses`).
      if (working.current || !escapeCloses(e)) return;
      close.current();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  const current = steps[step];
  const last = step === steps.length - 1;

  return (
    <div style={backdrop} role="presentation">
      <div ref={dialog} tabIndex={-1} role="dialog" aria-modal="true" aria-label={title} style={sheet}>
        <div style={{ fontSize: 15, fontWeight: 600, padding: "12px 16px", borderBottom: "1px solid #e5e7eb" }}>{title}</div>
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          {result === null && (
            <ol aria-label="Schritte" style={{ listStyle: "none", margin: 0, padding: "12px 16px", width: 160, borderRight: "1px solid #e5e7eb", flexShrink: 0 }}>
              {steps.map((s, i) => (
                <li
                  key={s.id}
                  aria-current={i === step ? "step" : undefined}
                  style={{ padding: "3px 0", fontWeight: i === step ? 700 : 400, color: i > step ? "#9ca3af" : "#111827" }}
                >
                  {i + 1}. {s.title}
                </li>
              ))}
            </ol>
          )}
          <div style={{ flex: 1, padding: 16, overflowY: "auto", minWidth: 0 }}>{result ?? current?.content}</div>
        </div>
        <div style={footer}>
          {result === null && error && (
            <div role="alert" style={banner}>
              {error}
            </div>
          )}
          {result !== null ? (
            <button type="button" onClick={onClose}>
              Schließen
            </button>
          ) : (
            <>
              <button type="button" disabled={busy} onClick={onClose}>
                Abbrechen
              </button>
              <button type="button" disabled={busy || step === 0} onClick={() => onStep(step - 1)}>
                Zurück
              </button>
              {last ? (
                <button type="button" disabled={busy || !current?.canNext} onClick={onSubmit}>
                  {submitLabel}
                </button>
              ) : (
                <button type="button" disabled={busy || !current?.canNext} onClick={() => onStep(step + 1)}>
                  Weiter
                </button>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
