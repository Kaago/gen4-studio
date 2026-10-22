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
  placeholder?: string;
  label: string;
  style?: CSSProperties;
}

/**
 * An input that writes on blur or Enter, never per keystroke. After a commit the field shows
 * `value` again until the owner passes the new one (a refused or clamped input thus never
 * lingers). Text still being typed when the field unmounts (the inspector swaps its content on
 * hover without blurring) is committed instead of lost.
 */
export function DraftInput({ value, onCommit, type = "text", min, max, step, placeholder, label, style }: Props) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);

  const latest = useRef({ draft, value, onCommit });
  latest.current = { draft, value, onCommit };

  useEffect(
    () => () => {
      const { draft, value, onCommit } = latest.current;
      if (draft !== value) onCommit(draft);
    },
    [],
  );

  const commit = () => {
    if (draft !== value) onCommit(draft);
    setDraft(value);
  };

  return (
    <input
      type={type}
      inputMode={type === "number" ? (step === undefined ? "numeric" : "decimal") : undefined}
      min={min}
      max={max}
      step={step}
      value={draft}
      placeholder={placeholder}
      aria-label={label}
      title={label}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        else if (e.key === "Escape") setDraft(value);
      }}
      style={{ boxSizing: "border-box", padding: "2px 6px", ...style }}
    />
  );
}
