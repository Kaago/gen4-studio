import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Ref } from "react";
import { useTranslation } from "react-i18next";
import { validateText } from "../../api";
import type { ProblemDto } from "../../api";
import { Notice } from "../../ui/primitives";
import { segments, toTextForm } from "./textModel";
import type { SegmentKind } from "./textModel";
import styles from "./MessageEditor.module.css";

/** Quiet time after the last keystroke before a text is committed (a blur commits at once). */
const COMMIT_MS = 500;
/** Quiet time before the length check asks the backend. */
const CHECK_MS = 300;

/** The code Shift+Enter writes: clear (▼), the next page starts empty. */
const CLEAR = "\\r";

const SEGMENT_SHOWN: Partial<Record<SegmentKind, string>> = { newline: "⏎", clear: "▼", scroll: "▽" };

export interface MessageEditorProps {
  value: string;
  /** Resolves to whether the backend took the text; a refused text is dropped for the stored one. */
  onCommit: (text: string) => Promise<boolean>;
  /** Hears every change of the field's text (also the stored one coming back), for a preview that follows the typing. */
  onDraft?: (text: string) => void;
  /** Hears when the field gets and loses the focus. */
  onFocusChange?: (focused: boolean) => void;
  /** The text area itself (Enter in the message table jumps here). */
  fieldRef?: Ref<HTMLTextAreaElement>;
  /** The field's accessible name („Text der Nachricht“ unless a form has several texts: „Vor dem Kampf“). */
  label?: string;
  /** Shown but not editable (a locked trainer's battle texts). */
  disabled?: boolean;
}

/**
 * The field of one message. It keeps the typed text and commits it on blur, after `COMMIT_MS` of
 * rest and when it goes away (a switch to another message or archive): `onCommit` resolves to
 * whether the backend took it, and a refused text is dropped for the stored one. While the archive
 * is read again (after a commit, undo, redo) the field follows the stored text unless there is typing the backend does not have yet.
 * Enter writes a line break (`\n`), Shift+Enter a new page (`\r`); ⌘S commits before the app saves.
 */
export function MessageEditor({ value, onCommit, onDraft, onFocusChange, fieldRef, label, disabled = false }: MessageEditorProps) {
  const { t } = useTranslation("workbench");
  const [draft, setDraft] = useState(value);
  const draftRef = useRef(draft);
  // The text the backend has, or is about to have: what the draft is compared with.
  const sent = useRef(value);
  const latest = useRef({ value, onCommit, onDraft });
  latest.current = { value, onCommit, onDraft };
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);
  const field = useRef<HTMLTextAreaElement | null>(null);
  const caret = useRef<number | null>(null);
  const [problems, setProblems] = useState<ProblemDto[]>([]);

  useEffect(() => {
    if (draftRef.current === sent.current) {
      draftRef.current = value;
      setDraft(value);
    }
    sent.current = value;
  }, [value]);

  // The preview follows the field.
  useEffect(() => latest.current.onDraft?.(draft), [draft]);

  const flush = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
    const text = draftRef.current;
    if (text === sent.current) return;
    sent.current = text;
    void latest.current.onCommit(text).then((ok) => {
      if (ok || !mounted.current) return;
      // Refused: the field shows what is stored again.
      sent.current = latest.current.value;
      draftRef.current = sent.current;
      setDraft(sent.current);
    });
  }, []);

  useEffect(() => {
    // Set again here: StrictMode's simulated unmount would otherwise leave it false for good.
    mounted.current = true;
    return () => {
      mounted.current = false;
      flush();
    };
  }, [flush]);

  // A line break typed or pasted is written as `\n`, and the caret stays where it was typed.
  useLayoutEffect(() => {
    if (caret.current !== null && field.current) field.current.setSelectionRange(caret.current, caret.current);
    caret.current = null;
  }, [draft]);

  function change(raw: string, at: number) {
    const text = toTextForm(raw);
    caret.current = toTextForm(raw.slice(0, at)).length;
    draftRef.current = text;
    setDraft(text);
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = setTimeout(flush, COMMIT_MS);
  }

  // The length check, for the stored text on selection and for the draft while typing.
  useEffect(() => {
    let current = true;
    const check = setTimeout(() => {
      validateText(draft).then(
        (found) => current && setProblems(found),
        () => current && setProblems([]),
      );
    }, CHECK_MS);
    return () => {
      current = false;
      clearTimeout(check);
    };
  }, [draft]);

  const setField = useCallback(
    (el: HTMLTextAreaElement | null) => {
      field.current = el;
      if (typeof fieldRef === "function") fieldRef(el);
      else if (fieldRef) fieldRef.current = el;
    },
    [fieldRef],
  );

  const parts = useMemo(() => segments(draft), [draft]);
  return (
    <div className={styles.editor}>
      <textarea
        ref={setField}
        className={styles.field}
        value={draft}
        aria-label={label ?? t("messageEditor.label")}
        disabled={disabled}
        rows={5}
        spellCheck={false}
        onChange={(e) => change(e.target.value, e.target.selectionStart)}
        onFocus={() => onFocusChange?.(true)}
        onBlur={() => {
          flush();
          onFocusChange?.(false);
        }}
        onKeyDown={(e) => {
          // Cmd+S saves the project (the app's handler, further up): what is typed has to be written first.
          if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") flush();
          // Shift+Enter starts a new page: the clear code goes where the caret is (over a selection).
          if (e.key === "Enter" && e.shiftKey && !e.metaKey && !e.ctrlKey && !e.altKey) {
            e.preventDefault();
            const el = e.currentTarget;
            const start = el.selectionStart;
            change(el.value.slice(0, start) + CLEAR + el.value.slice(el.selectionEnd), start + CLEAR.length);
          }
        }}
      />
      {parts.length > 0 && (
        <div aria-label={t("messageEditor.codes")} className={styles.codes}>
          {parts.map((s, i) => (
            <span key={i} data-segment={s.kind} title={s.kind === "text" ? undefined : s.text} className={styles[s.kind]}>
              {SEGMENT_SHOWN[s.kind] ?? s.text}
            </span>
          ))}
        </div>
      )}
      {problems.length > 0 && (
        <Notice kind="warning">
          <ul aria-label={t("messageEditor.warnings")} className={styles.warnings}>
            {problems.map((p, i) => (
              <li key={i}>{t("messageEditor.warning", { line: p.line, message: p.message })}</li>
            ))}
          </ul>
        </Notice>
      )}
    </div>
  );
}
