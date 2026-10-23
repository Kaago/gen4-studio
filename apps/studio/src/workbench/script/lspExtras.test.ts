// @vitest-environment jsdom
import { EditorState, type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { LSPClient, languageServerExtensions, type LSPClientExtension } from "@codemirror/lsp-client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FakeLspServer } from "./fakeLspServer";
import { codeLenses, documentSymbols, inlayHints, lspExtras } from "./lspExtras";
import { rotomLanguage } from "./rotomLanguage";

const URI = "file:///p/expanded/scripts/0001.rotom";
const range = (line: number, from = 0, to = 0) => ({ start: { line, character: from }, end: { line, character: to } });

describe("documentSymbols", () => {
  const fakeClient = (result: unknown) => {
    const request = vi.fn(async () => result);
    const sync = vi.fn();
    return { client: { request, sync } as unknown as LSPClient, request, sync };
  };

  it("lists_the_symbols_with_one_based_lines", async () => {
    const { client, request, sync } = fakeClient([
      { name: "Main", detail: "#1", kind: 12, range: range(0, 0, 20), selectionRange: range(0, 7, 11) },
      { name: "Second", kind: 12, range: range(4, 0, 9), selectionRange: range(4, 7, 13) },
    ]);
    await expect(documentSymbols(client, URI)).resolves.toEqual([
      { name: "Main", detail: "#1", line: 1 },
      { name: "Second", detail: undefined, line: 5 },
    ]);
    expect(sync).toHaveBeenCalled();
    expect(request).toHaveBeenCalledWith("textDocument/documentSymbol", { textDocument: { uri: URI } });
  });

  it("flattens_children_and_reads_flat_symbol_information", async () => {
    const nested = fakeClient([
      { name: "Outer", range: range(0), selectionRange: range(0), children: [{ name: "Inner", range: range(2), selectionRange: range(2) }] },
    ]);
    expect((await documentSymbols(nested.client, URI)).map((s) => [s.name, s.line])).toEqual([
      ["Outer", 1],
      ["Inner", 3],
    ]);
    const flat = fakeClient([{ name: "Flat", kind: 12, location: { uri: URI, range: range(6) } }]);
    expect(await documentSymbols(flat.client, URI)).toEqual([{ name: "Flat", detail: undefined, line: 7 }]);
  });

  it("returns_an_empty_list_when_the_server_has_no_symbols", async () => {
    expect(await documentSymbols(fakeClient(null).client, URI)).toEqual([]);
  });
});

describe("lspExtras capabilities", () => {
  it("advertise_the_features_the_extras_use", () => {
    const caps = (lspExtras as LSPClientExtension).clientCapabilities as { textDocument: Record<string, unknown> };
    expect(Object.keys(caps.textDocument).sort()).toEqual(["codeLens", "documentSymbol", "inlayHint"]);
  });
});

describe("inlayHints and codeLenses", () => {
  let server: FakeLspServer;
  let client: LSPClient | undefined;
  let host: HTMLElement;
  let view: EditorView | null;

  beforeEach(() => {
    vi.useFakeTimers();
    server = new FakeLspServer();
    server.capabilities = { inlayHintProvider: true, codeLensProvider: { resolveProvider: true } };
    host = document.createElement("div");
    document.body.append(host);
    view = null;
  });
  afterEach(() => {
    view?.destroy();
    host.remove();
    client?.disconnect();
    vi.useRealTimers();
  });

  /** Connects a client (after the test set the server's capabilities) and mounts an editor with it. */
  function mount(doc: string, extra: Extension[]) {
    client = new LSPClient({ extensions: [...languageServerExtensions(), lspExtras] }).connect(server);
    view = new EditorView({
      parent: host,
      state: EditorState.create({ doc, extensions: [rotomLanguage, client.plugin(URI, "rotom"), ...extra] }),
    });
    return view;
  }
  const settle = (ms: number) => vi.advanceTimersByTimeAsync(ms);

  it("shows_inlay_hints_at_the_server_position_after_the_debounce", async () => {
    server.methods["textDocument/inlayHint"] = () => [
      { position: { line: 1, character: 9 }, label: "msg:", paddingRight: true },
      { position: { line: 1, character: 15 }, label: [{ value: "a" }, { value: "b" }] },
    ];
    const v = mount('script Main #1:\n\tMessage 1 "Hi"\n', [inlayHints]);
    await settle(299);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(0);
    await settle(1);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(1);
    expect(server.calls("textDocument/inlayHint")[0]!.params).toMatchObject({ textDocument: { uri: URI }, range: { start: { line: 0, character: 0 } } });
    const hints = [...v.contentDOM.querySelectorAll(".cm-inlay-hint")].map((e) => e.textContent);
    expect(hints).toEqual(["msg:", "ab"]);
    // The first hint sits before the argument (column 9), the second at the end of the line.
    const line2 = v.contentDOM.querySelectorAll(".cm-line")[1]!;
    expect(line2.textContent).toBe('\tMessage msg:1 "Hi"ab');
  });

  it("requests_once_for_a_burst_of_edits_and_keeps_hints_in_place_while_typing", async () => {
    server.methods["textDocument/inlayHint"] = () => [{ position: { line: 0, character: 6 }, label: "h" }];
    const v = mount("script Main", [inlayHints]);
    await settle(300);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(1);
    v.dispatch({ changes: { from: 0, insert: "X" } });
    v.dispatch({ changes: { from: 0, insert: "Y" } });
    // The decoration moved with the text before the new response arrives.
    expect(v.contentDOM.querySelector(".cm-line")!.textContent).toBe("YXscripth Main");
    await settle(299);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(1);
    await settle(1);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(2);
  });

  it("maps_hint_positions_through_edits_made_while_the_request_was_open", async () => {
    server.methods["textDocument/inlayHint"] = () => [{ position: { line: 0, character: 6 }, label: "h" }];
    const v = mount("script Main", [inlayHints]);
    const original = server.methods["textDocument/inlayHint"]!;
    server.methods["textDocument/inlayHint"] = (p) => {
      // Typed after the request left, before the answer arrives.
      v.dispatch({ changes: { from: 0, insert: "ZZ" } });
      return original(p);
    };
    await settle(300);
    expect(v.contentDOM.querySelector(".cm-line")!.textContent).toBe("ZZscripth Main");
  });

  it("asks_nothing_when_the_server_has_no_inlay_hint_provider", async () => {
    server.capabilities = {};
    mount("script Main", [inlayHints]);
    await settle(400);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(0);
  });

  it("shows_code_lenses_above_their_line_and_resolves_lenses_without_a_command", async () => {
    server.methods["textDocument/codeLens"] = () => [
      { range: range(0), command: { title: "2 Verweise", command: "" } },
      { range: range(0), data: 7 },
      { range: range(2), command: { title: "1 Verweis", command: "" } },
    ];
    server.methods["codeLens/resolve"] = (lens: { data: number; range: unknown }) => ({ ...lens, command: { title: "Test", command: "" } });
    const v = mount("script Main #1:\n\tEnd\nscript Other #2:\n\tEnd\n", [codeLenses]);
    await settle(300);
    expect(server.calls("codeLens/resolve")).toHaveLength(1);
    const lenses = [...v.contentDOM.querySelectorAll(".cm-code-lens")].map((e) => e.textContent);
    expect(lenses).toEqual(["2 Verweise | Test", "1 Verweis"]);
    // A block widget sits before its line.
    const firstLens = v.contentDOM.querySelector(".cm-code-lens")!;
    const firstLine = v.contentDOM.querySelectorAll(".cm-line")[0]!;
    expect(firstLens.compareDocumentPosition(firstLine) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("does_nothing_without_a_language_server_plugin", async () => {
    const plain = new EditorView({
      parent: host,
      state: EditorState.create({ doc: "script Main", extensions: [inlayHints, codeLenses] }),
    });
    await settle(400);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(0);
    plain.destroy();
  });

  it("drops_a_late_answer_for_a_view_that_was_destroyed", async () => {
    let answer!: (hints: unknown) => void;
    server.methods["textDocument/inlayHint"] = () => new Promise((resolve) => (answer = resolve));
    const v = mount("script Main", [inlayHints]);
    await settle(300);
    expect(server.calls("textDocument/inlayHint")).toHaveLength(1);
    const dispatch = vi.spyOn(v, "dispatch");
    v.destroy();
    view = null;
    answer([{ position: { line: 0, character: 0 }, label: "late" }]);
    await settle(10);
    expect(dispatch).not.toHaveBeenCalled();
  });
});
