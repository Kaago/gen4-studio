import { useEffect, useRef, useState } from "react";

/**
 * The draft behaviour shared by the text and number fields: the text being typed lives here, the owner
 * hears about it only through `onCommit` (on `flush`, and once more when the field unmounts mid-typing), and after a
 * commit the field shows `value` again until the owner passes a new one. A refused entry thus never lingers in the field.
 */
export function useDraft(value: string, onCommit: (text: string) => void) {
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

  return {
    text: draft ?? value,
    edited: draft !== null && draft !== value,
    change: setDraft,
    /** Commits what was typed (if it differs) and shows `value` again. */
    flush() {
      if (draft !== null && draft !== value) onCommit(draft);
      setDraft(null);
      // The field may unmount before it renders again: nothing is left to commit.
      latest.current.draft = null;
    },
    /** Takes the typed text back. */
    revert() {
      setDraft(null);
      latest.current.draft = null;
    },
  };
}
