// @vitest-environment jsdom
import { undo } from "@codemirror/commands";
import { LSPClient, languageServerExtensions } from "@codemirror/lsp-client";
import type { Diagnostic } from "@codemirror/lint";
import { EditorView } from "@codemirror/view";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ScriptDocuments, type ScriptDocument } from "./documents";
import { FakeLspServer } from "./fakeLspServer";
import { ScriptEditor } from "./ScriptEditor";
import { lspExtras } from "./lspExtras";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const uriOf = (file: number) => `file:///p/expanded/scripts/${String(file).padStart(4, "0")}.rotom`;

describe("ScriptEditor", () => {
  let container: HTMLElement;
  let root: Root;
  let docs: ScriptDocuments;
  const onChange = vi.fn();
  const onSave = vi.fn();
  const onDiagnostics = vi.fn();

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    docs = new ScriptDocuments();
    onChange.mockReset();
    onSave.mockReset();
    onDiagnostics.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function render(doc: ScriptDocument, extra: Partial<Parameters<typeof ScriptEditor>[0]> = {}) {
    act(() =>
      root.render(
        <ScriptEditor
          doc={doc}
          language="rotom"
          lsp={null}
          uri={uriOf(doc.file)}
          onChange={onChange}
          onSave={onSave}
          onDiagnostics={onDiagnostics}
          theme="light"
          {...extra}
        />,
      ),
    );
  }
  const lastDiagnostics = () => onDiagnostics.mock.calls[onDiagnostics.mock.calls.length - 1]![0] as Diagnostic[];
  const view = () => EditorView.findFromDOM(container.querySelector(".cm-editor") as HTMLElement)!;
  const typeAt = (pos: number, text: string) => act(() => view().dispatch({ changes: { from: pos, insert: text }, userEvent: "input.type" }));

  it("mounts_the_document_text_with_rotom_highlighting", () => {
    render(docs.open(1, "script", "script Main #1:\n\tEnd\n"));
    expect(container.querySelector(".cm-editor")).not.toBeNull();
    expect(view().state.doc.toString()).toBe("script Main #1:\n\tEnd\n");
    expect(container.querySelectorAll(".cm-line")).toHaveLength(3);
  });

  it("uses_json_for_a_level_script", () => {
    render(docs.open(2, "levelScript", '{"a": 1}'), { language: "json" });
    expect(view().state.doc.toString()).toBe('{"a": 1}');
  });

  it("reports_typing_with_the_new_text_and_updates_the_document", () => {
    const doc = docs.open(1, "script", "script Main #1:\n");
    render(doc);
    typeAt(0, "// ");
    expect(onChange).toHaveBeenCalledWith("// script Main #1:\n");
    expect(doc.text).toBe("// script Main #1:\n");
    expect(docs.dirty()).toEqual([1]);
    expect(doc.cursor).toBe(view().state.selection.main.head);
    expect(doc.state).toBe(view().state);
  });

  it("calls_onSave_for_the_save_shortcut_and_not_onChange", () => {
    render(docs.open(1, "script", "x"));
    const content = container.querySelector(".cm-content") as HTMLElement;
    act(() => {
      content.dispatchEvent(new KeyboardEvent("keydown", { key: "s", code: "KeyS", ctrlKey: true, bubbles: true, cancelable: true }));
    });
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("keeps_undo_per_document_across_a_switch_away_and_back", () => {
    const a = docs.open(1, "script", "alpha");
    const b = docs.open(2, "script", "beta");
    render(a);
    typeAt(5, "!");
    expect(view().state.doc.toString()).toBe("alpha!");
    render(b);
    expect(view().state.doc.toString()).toBe("beta");
    typeAt(0, "B");
    render(a);
    expect(view().state.doc.toString()).toBe("alpha!");
    act(() => void undo(view()));
    expect(view().state.doc.toString()).toBe("alpha");
    expect(onChange).toHaveBeenLastCalledWith("alpha");
    expect(a.text).toBe("alpha");
    // The other document kept its own history, too.
    render(b);
    expect(view().state.doc.toString()).toBe("Bbeta");
    act(() => void undo(view()));
    expect(view().state.doc.toString()).toBe("beta");
  });

  it("does_not_report_a_change_when_only_switching_documents", () => {
    const a = docs.open(1, "script", "alpha");
    const b = docs.open(2, "script", "beta");
    render(a);
    render(b);
    render(a);
    expect(onChange).not.toHaveBeenCalled();
    expect(docs.dirty()).toEqual([]);
  });

  it("restores_the_cursor_of_the_document_and_jumps_when_the_cursor_is_set_from_outside", () => {
    const doc = docs.open(1, "script", "one\ntwo\nthree\n");
    doc.cursor = 4;
    render(doc);
    expect(view().state.selection.main.head).toBe(4);
    doc.cursor = 8;
    render(doc);
    expect(view().state.selection.main.head).toBe(8);
  });

  it("shows_text_that_was_replaced_from_outside_without_reporting_it_as_an_edit", () => {
    const doc = docs.open(1, "script", "old");
    render(doc);
    docs.setText(1, "new text");
    render(doc);
    expect(view().state.doc.toString()).toBe("new text");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("does_not_treat_crlf_text_as_an_outside_change", () => {
    const doc = docs.open(1, "script", "a\r\nb");
    render(doc);
    render(doc);
    expect(onChange).not.toHaveBeenCalled();
    expect(doc.text).toBe("a\r\nb");
  });

  it("switches_the_theme_without_losing_the_text", () => {
    const doc = docs.open(1, "script", "keep");
    render(doc, { theme: "light" });
    const before = view();
    render(doc, { theme: "dark" });
    expect(view()).toBe(before);
    expect(view().state.doc.toString()).toBe("keep");
    expect(view().state.facet(EditorView.darkTheme)).toBe(true);
  });

  describe("with a language server", () => {
    let server: FakeLspServer;
    let client: LSPClient;
    beforeEach(() => {
      server = new FakeLspServer();
      client = new LSPClient({ extensions: [...languageServerExtensions(), lspExtras] }).connect(server);
    });
    afterEach(() => {
      act(() => root.unmount());
      client.disconnect();
      root = createRoot(container);
    });

    it("sets_a_server_diagnostic_and_reports_it_through_onDiagnostics", async () => {
      server.methods["textDocument/didOpen"] = () => null;
      const doc = docs.open(1, "script", "script Main #1:\n\tBogus\n");
      render(doc, { lsp: client });
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
      expect(server.calls("textDocument/didOpen")[0]!.params).toMatchObject({
        textDocument: { uri: uriOf(1), languageId: "rotom", version: 0, text: "script Main #1:\n\tBogus\n" },
      });
      act(() =>
        server.notify("textDocument/publishDiagnostics", {
          uri: uriOf(1),
          version: 0,
          diagnostics: [{ range: { start: { line: 1, character: 1 }, end: { line: 1, character: 6 } }, message: "unknown command Bogus", severity: 1 }],
        }),
      );
      const last = lastDiagnostics();
      expect(last).toHaveLength(1);
      expect(last[0]).toMatchObject({ from: 17, to: 22, severity: "error", message: "unknown command Bogus" });
    });

    it("reports_the_stored_diagnostics_of_a_document_when_it_is_shown_again", async () => {
      const a = docs.open(1, "script", "script A #1:\n\tBogus\n");
      const b = docs.open(2, "script", "script B #1:\n");
      render(a, { lsp: client });
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
      act(() =>
        server.notify("textDocument/publishDiagnostics", {
          uri: uriOf(1),
          version: 0,
          diagnostics: [{ range: { start: { line: 1, character: 1 }, end: { line: 1, character: 6 } }, message: "bad", severity: 2 }],
        }),
      );
      render(b, { lsp: client });
      expect(lastDiagnostics()).toEqual([]);
      render(a, { lsp: client });
      expect(lastDiagnostics().map((d) => d.message)).toEqual(["bad"]);
    });

    it("opens_and_closes_the_file_on_the_server_as_documents_are_switched", async () => {
      const a = docs.open(1, "script", "a");
      const b = docs.open(2, "script", "b");
      render(a, { lsp: client });
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
      render(b, { lsp: client });
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(2));
      expect(server.calls("textDocument/didClose").map((m) => (m.params as { textDocument: { uri: string } }).textDocument.uri)).toEqual([uriOf(1)]);
    });

    it("attaches_a_client_that_arrives_later_without_recreating_the_editor", async () => {
      const doc = docs.open(1, "script", "x");
      render(doc, { lsp: null });
      const before = view();
      render(doc, { lsp: client });
      expect(view()).toBe(before);
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
      // A rerender with the same client does not reopen the file.
      render(doc, { lsp: client });
      await Promise.resolve();
      expect(server.calls("textDocument/didOpen")).toHaveLength(1);
      expect(server.calls("textDocument/didClose")).toHaveLength(0);
    });

    it("sends_edits_to_the_server_as_didChange", async () => {
      const doc = docs.open(1, "script", "abc");
      render(doc, { lsp: client });
      await vi.waitFor(() => expect(server.calls("textDocument/didOpen")).toHaveLength(1));
      typeAt(3, "d");
      await vi.waitFor(() => expect(server.calls("textDocument/didChange")).toHaveLength(1), { timeout: 2000 });
      expect(server.calls("textDocument/didChange")[0]!.params).toMatchObject({ textDocument: { uri: uriOf(1), version: 1 }, contentChanges: [{ text: "abcd" }] });
    });
  });
});
