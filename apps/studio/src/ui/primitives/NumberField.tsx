import { useEffect, useState } from "react";
import { Field } from "@base-ui/react/field";
import { NumberField as BaseNumberField } from "@base-ui/react/number-field";
import { Icon } from "./Icon";
import { checkNumber, formatNumber, parseNumber } from "./numberParse";
import type { NumberBase } from "./numberParse";
import { Segmented } from "./Segmented";
import { useDraft } from "./useDraft";
import styles from "./NumberField.module.css";

export type NumberFieldProps = {
  /** Always the accessible name; shown above the field (and then the scrub handle) only with `showLabel`. */
  label: string;
  value: number;
  /** Called with a valid, in-range number on blur or Enter, after a step, and when a scrub ends; never per keystroke. */
  onCommit: (value: number) => void;
  min?: number;
  max?: number;
  base?: NumberBase;
  /** The width of the value in bits; the largest value is the smaller of `max` and `2^bits − 1`. */
  bits?: 8 | 16 | 32;
  step?: number;
  hint?: string;
  disabled?: boolean;
  /** Shows the Dez | Hex switch. */
  showBase?: boolean;
  showLabel?: boolean;
};

/** The largest value the field takes: `max` and the bit width, whichever is smaller. */
function ceiling(max: number | undefined, bits: number | undefined): number | undefined {
  const byBits = bits === undefined ? undefined : 2 ** bits - 1;
  if (max === undefined) return byBits;
  return byBits === undefined ? max : Math.min(max, byBits);
}

const BASES = [
  { value: "dec", label: "Dez" },
  { value: "hex", label: "Hex" },
] as const;

/**
 * A whole-number field. What is typed stays a draft until blur or Enter; an entry that is unreadable or outside
 * `min..max` (or the bit width) is refused with the bound named, keeps the old value, and is never clamped to the bound.
 *
 * Base UI's `NumberField` supplies the scrub area (drag the label or the grip). Its own text handling would clamp on
 * blur and formats only decimals, so the input's change, key, paste and value are taken over: Base UI's value changes
 * are listened to for the `scrub` reason only, and `allowOutOfRange` is set so it never rewrites typed text either.
 */
export function NumberField({ label, value, onCommit, min = 0, max, base: baseProp = "dec", bits, step = 1, hint, disabled, showBase, showLabel }: NumberFieldProps) {
  const [chosenBase, setChosenBase] = useState<NumberBase | null>(null);
  const base = chosenBase ?? baseProp;
  const top = ceiling(max, bits);

  const [error, setError] = useState<string | null>(null);
  // The value while a scrub is under way; null otherwise.
  const [scrub, setScrub] = useState<number | null>(null);
  // A new value or another base makes an old refusal moot.
  useEffect(() => setError(null), [value, base]);

  const draft = useDraft(formatNumber(value, base), (text) => {
    const checked = checkNumber(text, base, min, top);
    if (!checked.ok) {
      setError(checked.error);
      return;
    }
    setError(null);
    if (checked.value !== value) onCommit(checked.value);
  });

  const stepBy = (direction: 1 | -1, factor: number) => {
    const from = parseNumber(draft.text, base) ?? value;
    const next = from + direction * step * factor;
    // A step is not an entry: at a bound it simply stops.
    if (next < min || (top !== undefined && next > top)) return;
    draft.revert();
    setError(null);
    if (next !== value) onCommit(next);
  };

  // Base UI scrubs in fractions of `step` (a quarter per pixel, see below); what is shown and committed is whole steps
  // counted from where the drag began, so the value moves once per 4 px and never past a bound.
  const snapScrub = (v: number) => {
    const snapped = value + Math.trunc((v - value) / step) * step;
    return Math.min(top ?? snapped, Math.max(min, snapped));
  };
  const invalid = error !== null;
  const shown = scrub ?? value;
  // Dragging the label scrubs the value; without a visible label a small grip in front of the input does.
  const scrubArea = (
    <BaseNumberField.ScrubArea data-scrub="" className={showLabel ? styles.scrubLabel : styles.scrubGrip} pixelSensitivity={1}>
      {showLabel ? <Field.Label className={styles.label}>{label}</Field.Label> : <Icon name="grip" />}
      <BaseNumberField.ScrubAreaCursor className={styles.cursor}>
        <Icon name="scrub" />
      </BaseNumberField.ScrubAreaCursor>
    </BaseNumberField.ScrubArea>
  );
  return (
    <Field.Root className={styles.field} invalid={invalid} disabled={disabled}>
      <BaseNumberField.Root
        className={styles.root}
        value={shown}
        min={min}
        max={top}
        step={step / 4}
        smallStep={step / 4}
        largeStep={(step * 10) / 4}
        allowOutOfRange
        disabled={disabled}
        onValueChange={(next, details) => {
          if (details.reason === "scrub") setScrub(next);
        }}
        onValueCommitted={(next, details) => {
          if (details.reason !== "scrub") return;
          setScrub(null);
          if (next === null) return;
          const final = snapScrub(next);
          if (final !== value) onCommit(final);
        }}
      >
        {showLabel && scrubArea}
        <div className={styles.row}>
          {!showLabel && scrubArea}
          <BaseNumberField.Input
            className={base === "hex" ? `${styles.input} ${styles.mono}` : styles.input}
            aria-label={label}
            aria-roledescription="Zahlfeld"
            data-draft=""
            inputMode={base === "hex" ? "text" : "numeric"}
            value={scrub !== null ? formatNumber(snapScrub(scrub), base) : draft.text}
            onChange={(e) => {
              e.preventBaseUIHandler();
              setError(null);
              draft.change(e.currentTarget.value);
            }}
            onPaste={(e) => e.preventBaseUIHandler()}
            onBlur={() => draft.flush()}
            onKeyDown={(e) => {
              // Base UI would block the letters of a hex entry and step on its own terms.
              e.preventBaseUIHandler();
              if (e.key === "Enter") {
                // Committing typed text uses the key (a wizard around the field must not move on as well); an untouched field leaves it.
                if (draft.edited) e.preventDefault();
                e.currentTarget.blur();
              } else if (e.key === "Escape") {
                if (draft.edited) {
                  // Taking back the typed text uses the key: a dialog around the field must not close as well.
                  e.preventDefault();
                  draft.revert();
                }
                setError(null);
              } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
                e.preventDefault();
                stepBy(e.key === "ArrowUp" ? 1 : -1, e.shiftKey ? 10 : 1);
              }
            }}
          />
          {showBase && (
            <Segmented
              size="s"
              ariaLabel={`Zahlenbasis von ${label}`}
              options={BASES}
              value={base}
              onChange={(next) => {
                draft.revert();
                setChosenBase(next);
              }}
            />
          )}
        </div>
      </BaseNumberField.Root>
      {hint && <Field.Description className={styles.hint}>{hint}</Field.Description>}
      {error !== null && (
        <Field.Error match className={styles.error}>
          {error}
        </Field.Error>
      )}
    </Field.Root>
  );
}
