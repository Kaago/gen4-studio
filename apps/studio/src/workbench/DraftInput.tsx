import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";

interface Props {
  /** The text the field shows when nothing is being typed. */
  value: string;
  /** Called with the typed text on blur or Enter when it differs from `value`. */
  onCommit: (text: string) => void;
  type?: "text" | "number";
  min?: number;
  max?: number;
  /** Arrow-key step of a number field; setting it also asks for a decimal keyboard. */
  step?: number;
  /** Longest text the field takes while typing (text fields). */
  maxLength?: number;
  /** Regular expression the text should match (a hint to the browser; the owner still checks what it commits). */
  pattern?: string;
  placeholder?: string;
  label: string;
  disabled?: boolean;
  style?: CSSProperties;
}

/**
 * An input that writes on blur or Enter, never per keystroke. After a commit the field shows
 * `value` again until the owner passes the new one (a refused or clamped input thus never
 * lingers). Text still being typed when the field unmounts (the inspector swaps its content on
 * hover without blurring) is committed instead of lost; text already committed is not committed
 * again, even when the field unmounts before it has shown the owner's new value.
 */
export function DraftInput({ value, onCommit, type = "text", min, max, step, maxLength, pattern, placeholder, label, disabled, style }: Props) {
  // The text being typed; null while nothing is (the field shows `value`).
  const [draft, setDraft] = useState<string | null>(null);
  useEffect(() => setDraft(null), [value]);

  const latest = useRef({ draft, value, onCommit });
  latest.current = { draft, value, onCommit };

  useEffect(
    () => () => {
      const { draft, value, onCommit } = latest.current;
      if (draft !== null && draft !== value) onCommit(draft);
    },
    [],
  );

  const commit = () => {
    if (draft !== null && draft !== value) onCommit(draft);
    setDraft(null);
    // The field may unmount before it renders again (a click on "Weiter" right after typing): nothing is left to commit.
    latest.current.draft = null;
  };

  return (
    <input
      type={type}
      inputMode={type === "number" ? (step === undefined ? "numeric" : "decimal") : undefined}
      min={min}
      max={max}
      step={step}
      maxLength={maxLength}
      pattern={pattern}
      value={draft ?? value}
      placeholder={placeholder}
      aria-label={label}
      disabled={disabled}
      title={label}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          // Committing typed text uses the key (a wizard around the field must not move on as well); an untouched field leaves it.
          if (draft !== null && draft !== value) e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape" && draft !== null && draft !== value) {
          // Taking back the typed text uses the key: a dialog around the field must not close as well.
          e.preventDefault();
          setDraft(null);
        }
      }}
      data-draft=""
      style={{ boxSizing: "border-box", padding: "2px 6px", ...style }}
    />
  );
}
