import { indentWithTab } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { indentUnit } from "@codemirror/language";
import { forEachDiagnostic, setDiagnosticsEffect, type Diagnostic } from "@codemirror/lint";
import type { LSPClient } from "@codemirror/lsp-client";
import { Annotation, Compartment, EditorState, Transaction, type Extension } from "@codemirror/state";
import { EditorView, keymap, type ViewUpdate } from "@codemirror/view";
import { basicSetup } from "codemirror";
import { useEffect, useLayoutEffect, useMemo, useRef, type JSX } from "react";
import type { ScriptDocument } from "./documents";
import { codeLenses, inlayHints } from "./lspExtras";
import { rotomLanguage } from "./rotomLanguage";
import { ROTOM_THEME } from "./rotomTheme";
import styles from "./ScriptEditor.module.css";

export interface ScriptEditorProps {
  doc: ScriptDocument;
  language: "rotom" | "json";
  /** The connected client, or null while there is none (the editor works without language services). */
  lsp: LSPClient | null;
  /** `file://<root>/expanded/scripts/NNNN.rotom`; the language server's name for the file. */
  uri: string;
  /** The new full text after a user edit. The editor has already stored it in `doc.text`. */
  onChange: (text: string) => void;
  /** The diagnostics of the shown document: after each change of them, and when a document is shown. */
  onDiagnostics: (d: Diagnostic[]) => void;
  theme: "hell" | "dunkel";
}

const baseTheme = EditorView.theme({
  "&": { height: "100%" },
  ".cm-scroller": { overflow: "auto" },
});

const ROTOM: Extension = [rotomLanguage, indentUnit.of("\t")];
const NO_LSP: Extension = [];

/** The language server extension of one file; one object per client and uri, so a stored state is not reconfigured needlessly. */
const lspExtensions = new WeakMap<LSPClient, Map<string, Extension>>();
function lspExtension(lsp: LSPClient | null, uri: string, language: ScriptEditorProps["language"]): Extension {
  if (!lsp) return NO_LSP;
  let byUri = lspExtensions.get(lsp);
  if (!byUri) lspExtensions.set(lsp, (byUri = new Map()));
  const key = `${language}\0${uri}`;
  let ext = byUri.get(key);
  if (!ext) byUri.set(key, (ext = [lsp.plugin(uri, language), inlayHints, codeLenses]));
  return ext;
}

const languageSlot = new Compartment();
const themeSlot = new Compartment();
const lspSlot = new Compartment();

interface Wanted {
  language: Extension;
  theme: Extension;
  lsp: Extension;
}
const slots = (w: Wanted): [Compartment, Extension][] => [
  [languageSlot, w.language],
  [themeSlot, w.theme],
  [lspSlot, w.lsp],
];

/** Marks a transaction that applies text from outside (not a user edit): no `onChange`, no undo step. */
const External = Annotation.define<boolean>();

/** What one `ScriptEditor` instance lends to the editor states. The states outlive the component, so the callbacks are looked up by view. */
interface Bridge {
  update(update: ViewUpdate): void;
}
const bridges = new WeakMap<EditorView, Bridge>();

// Cmd+S is not bound here: the window's shortcut (`useShortcuts`) saves the store first and then the open scripts, and
// the editor must let the key through to it (a second save from here would run ahead of the store's).
const bridge: Extension = EditorView.updateListener.of((update) => bridges.get(update.view)?.update(update));

function createState(doc: ScriptDocument, wanted: Wanted): EditorState {
  return EditorState.create({
    doc: doc.text,
    selection: { anchor: Math.min(doc.cursor, doc.text.length) },
    extensions: [
      basicSetup,
      keymap.of([indentWithTab]),
      baseTheme,
      bridge,
      slots(wanted).map(([slot, ext]) => slot.of(ext)),
    ],
  });
}

const normalized = (text: string) => text.replace(/\r\n?/g, "\n");

/** The single change that turns `old` into `next` (common prefix and suffix kept), so history and selection map through it. */
function minimalChange(old: string, next: string): { from: number; to: number; insert: string } {
  const max = Math.min(old.length, next.length);
  let start = 0;
  while (start < max && old[start] === next[start]) start++;
  let endOld = old.length;
  let endNew = next.length;
  while (endOld > start && endNew > start && old[endOld - 1] === next[endNew - 1]) {
    endOld--;
    endNew--;
  }
  return { from: start, to: endOld, insert: next.slice(start, endNew) };
}

/** `state` with every compartment brought to the wanted extension; the same state when none differs. */
function reconfigured(state: EditorState, wanted: Wanted): EditorState {
  const effects = slots(wanted)
    .filter(([slot, ext]) => slot.get(state) !== ext)
    .map(([slot, ext]) => slot.reconfigure(ext));
  return effects.length ? state.update({ effects }).state : state;
}

function diagnosticsOf(state: EditorState): Diagnostic[] {
  const found: Diagnostic[] = [];
  forEachDiagnostic(state, (d) => found.push(d));
  return found;
}

/**
 * A CodeMirror editor for one script document at a time. Each document keeps its own `EditorState` in
 * `doc.state`, so switching away and back keeps its undo history, selection and folds. Edits are written to
 * `doc.text`, `doc.state` and `doc.cursor` as they happen; text set from outside (`doc.text` changed, `doc.cursor`
 * moved) is shown on the next render. Line endings are normalised to `\n` by the first edit.
 */
export function ScriptEditor(props: ScriptEditorProps): JSX.Element {
  const { doc, language, lsp, uri, theme } = props;
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  /** The document whose state the view shows. */
  const shown = useRef(doc);
  const latest = useRef(props);
  const wanted = useMemo<Wanted>(
    () => ({
      language: language === "rotom" ? ROTOM : json(),
      theme: ROTOM_THEME[theme],
      lsp: lspExtension(lsp, uri, language),
    }),
    [language, theme, lsp, uri],
  );
  const wantedRef = useRef(wanted);
  useLayoutEffect(() => {
    latest.current = props;
    wantedRef.current = wanted;
  });

  // The view lives as long as the component; documents come and go through `setState`.
  useEffect(() => {
    const first = latest.current.doc;
    shown.current = first;
    // A stored state may carry an older client, uri or theme: fix it before any plugin is created from it.
    first.state = reconfigured(first.state ?? createState(first, wantedRef.current), wantedRef.current);
    const view = new EditorView({ state: first.state, parent: host.current! });
    const report = () => latest.current.onDiagnostics(diagnosticsOf(view.state));
    bridges.set(view, {
      update(update) {
        const current = shown.current;
        current.state = update.state;
        current.cursor = update.state.selection.main.head;
        const external = update.transactions.some((tr) => tr.annotation(External));
        if (update.docChanged && !external) {
          current.text = update.state.doc.toString();
          latest.current.onChange(current.text);
        }
        if (update.transactions.some((tr) => tr.effects.some((e) => e.is(setDiagnosticsEffect)))) report();
      },
    });
    viewRef.current = view;
    report();
    return () => {
      bridges.delete(view);
      viewRef.current = null;
      view.destroy();
    };
  }, []);

  // After every render: show the document, bring configuration and text up to date.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    let switched = false;
    if (shown.current !== doc) {
      shown.current = doc;
      doc.state = reconfigured(doc.state ?? createState(doc, wantedRef.current), wantedRef.current);
      view.setState(doc.state);
      switched = true;
    }
    // The client, uri or theme of the shown document changed.
    const effects = slots(wantedRef.current)
      .filter(([slot, ext]) => slot.get(view.state) !== ext)
      .map(([slot, ext]) => slot.reconfigure(ext));
    if (effects.length) view.dispatch({ effects });

    const want = normalized(doc.text);
    const old = view.state.doc.toString();
    if (old !== want) {
      // `doc.cursor` is read before the dispatch: the update listener overwrites it with the mapped head.
      const jump = doc.cursor !== view.state.selection.main.head;
      view.dispatch({
        changes: minimalChange(old, want),
        selection: jump ? { anchor: Math.min(doc.cursor, want.length) } : undefined,
        scrollIntoView: jump,
        annotations: [External.of(true), Transaction.addToHistory.of(false)],
      });
    } else {
      const cursor = Math.min(doc.cursor, old.length);
      if (view.state.selection.main.head !== cursor) view.dispatch({ selection: { anchor: cursor }, scrollIntoView: true });
    }

    if (switched) latest.current.onDiagnostics(diagnosticsOf(view.state));
  });

  return <div ref={host} className={styles.host} />;
}
