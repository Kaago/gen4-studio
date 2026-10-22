import type { EditorState } from "@codemirror/state";

/**
 * One open script (a `.rotom` source or a level-script JSON). `state` is the editor's own state, kept
 * here so that switching to another document and back keeps the undo history, selection and folds.
 */
export interface ScriptDocument {
  file: number;
  kind: "script" | "levelScript";
  text: string;
  /** The text as last read from or written to disk; the document is dirty while `text` differs. */
  savedText: string;
  /** Set by `ScriptEditor` when the document was shown; null until then. */
  state: EditorState | null;
  /** Offset of the cursor, kept in step with `state` by the editor; set it to make the editor jump there. */
  cursor: number;
}

/** The open script documents, in the order they were opened. Plain mutable store; no change events. */
export class ScriptDocuments {
  private readonly docs = new Map<number, ScriptDocument>();

  /** Opens `file` with `text` from disk, or returns the document that is already open (its edits stay). */
  open(file: number, kind: ScriptDocument["kind"], text: string): ScriptDocument {
    const open = this.docs.get(file);
    if (open) return open;
    const doc: ScriptDocument = { file, kind, text, savedText: text, state: null, cursor: 0 };
    this.docs.set(file, doc);
    return doc;
  }

  get(file: number): ScriptDocument | undefined {
    return this.docs.get(file);
  }

  /** Replaces the text; a file that is not open is ignored (an editor can report a change after its document closed). */
  setText(file: number, text: string): void {
    const doc = this.docs.get(file);
    if (doc) doc.text = text;
  }

  /** Records that the current text is on disk. */
  markSaved(file: number): void {
    const doc = this.docs.get(file);
    if (doc) doc.savedText = doc.text;
  }

  close(file: number): void {
    this.docs.delete(file);
  }

  /** Files whose text differs from their saved text. */
  dirty(): number[] {
    return this.list()
      .filter((d) => d.text !== d.savedText)
      .map((d) => d.file);
  }

  list(): ScriptDocument[] {
    return [...this.docs.values()];
  }
}

/** The offset where 1-based `line` of `text` starts; the end of the text for a line beyond it, 0 for line 1 or less. */
export function lineOffset(text: string, line: number): number {
  let offset = 0;
  for (let n = 1; n < line; n++) {
    const next = text.indexOf("\n", offset);
    if (next < 0) return text.length;
    offset = next + 1;
  }
  return offset;
}

/**
 * The language server's name for a script file: `file://<root>/expanded/scripts/NNNN.rotom` (`.json` for a
 * level script). Case is kept. Characters are percent-encoded as the URL standard does for a file path (spaces,
 * non-ASCII), which is what the server's URL type writes back; parentheses stay as they are. `%`, `#` and `?`
 * in the root are encoded too, so they cannot start an escape, fragment or query.
 */
export function scriptUri(root: string, file: number, kind: ScriptDocument["kind"] = "script"): string {
  const name = `${String(file).padStart(4, "0")}.${kind === "script" ? "rotom" : "json"}`;
  return projectFileUri(root, `expanded/scripts/${name}`);
}

/** The `file://` URL of a project-relative path (forward slashes), encoded like `scriptUri`. */
export function projectFileUri(root: string, path: string): string {
  return new URL(`file://${encodedRoot(root)}/${path}`).href;
}

/** The language server's `rootUri`: the project root as `file://` URL, encoded like in `scriptUri`. */
export function rootUri(root: string): string {
  return new URL(`file://${encodedRoot(root)}`).href;
}

function encodedRoot(root: string): string {
  return root
    .replace(/\/+$/, "")
    .replace(/%/g, "%25")
    .replace(/#/g, "%23")
    .replace(/\?/g, "%3F");
}
