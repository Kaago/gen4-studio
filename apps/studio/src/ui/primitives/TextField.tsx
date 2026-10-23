import { Field } from "@base-ui/react/field";
import { useDraft } from "./useDraft";
import styles from "./TextField.module.css";

export type TextFieldProps = {
  /** Always the accessible name; shown above the field only with `showLabel`. */
  label: string;
  value: string;
  /** Called with the typed text on blur or Enter when it differs from `value`; never per keystroke. */
  onCommit: (value: string) => void;
  /**
   * Called with the text on every keystroke, for a search that filters while typing. It does not replace `onCommit`
   * (the owner still gets the commit on Enter or blur); a search field passes its setter to both.
   */
  onChange?: (value: string) => void;
  type?: "text" | "search";
  /** A sentence that says what is wrong and the way out; marks the field invalid. */
  error?: string;
  hint?: string;
  disabled?: boolean;
  placeholder?: string;
  /** A text area. Enter then breaks the line; ⌘/Ctrl+Enter or leaving the field commits. */
  multiline?: boolean;
  showLabel?: boolean;
};

/**
 * A text field that writes on blur or Enter, like `DraftInput`: Escape takes the typed text back (and uses the key, so a
 * dialog around the field stays open), and `data-draft` marks the field for `escapeCloses`.
 */
export function TextField({ label, value, onCommit, onChange, type = "text", error, hint, disabled, placeholder, multiline, showLabel }: TextFieldProps) {
  const draft = useDraft(value, onCommit);
  const cls = multiline ? `${styles.control} ${styles.multiline}` : styles.control;
  return (
    <Field.Root className={styles.field} invalid={error !== undefined} disabled={disabled}>
      {showLabel && <Field.Label className={styles.label}>{label}</Field.Label>}
      <Field.Control
        className={cls}
        render={multiline ? <textarea rows={3} /> : <input type={type} />}
        aria-label={label}
        value={draft.text}
        placeholder={placeholder}
        disabled={disabled}
        spellCheck={false}
        data-draft=""
        onChange={(e) => {
          draft.change(e.currentTarget.value);
          onChange?.(e.currentTarget.value);
        }}
        onBlur={draft.flush}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (!multiline || e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            e.currentTarget.blur();
          } else if (e.key === "Escape" && draft.edited) {
            // Taking back the typed text uses the key: a dialog around the field must not close as well.
            e.preventDefault();
            draft.revert();
          }
        }}
      />
      {hint && <Field.Description className={styles.hint}>{hint}</Field.Description>}
      {error !== undefined && (
        <Field.Error match className={styles.error}>
          {error}
        </Field.Error>
      )}
    </Field.Root>
  );
}
