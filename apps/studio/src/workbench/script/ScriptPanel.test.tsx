// @vitest-environment jsdom
import { setDiagnostics } from "@codemirror/lint";
import { EditorView } from "@codemirror/view";
import { ask } from "../../platform/dialogs";
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectInfo, SaveScriptDto, ScriptsStatusDto } from "../../api";
import { addScript, adoptScripts, lspSend, lspStart, lspStop, readScript, saveLevelScript, saveScript, scriptsStatus } from "../../api";
import { createEditBus } from "../../bus/editBus";
import type { EditBus, EditEvent } from "../../bus/editBus";
import type { Depth } from "../../ui/frame/shellState";
import { ProblemsStore } from "../problems/problems";
import { ScriptPanel } from "./ScriptPanel";
import { toolStatus } from "../../app/toolStatus";
import { scriptDirtyStore } from "./scriptDirtyStore";
import { useScripts } from "./useScripts";
import type { ScriptsApi } from "./useScripts";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  scriptsStatus: vi.fn(),
  adoptScripts: vi.fn(),
  readScript: vi.fn(),
  saveScript: vi.fn(),
  saveLevelScript: vi.fn(),
  addScript: vi.fn(),
  lspStart: vi.fn(),
  lspSend: vi.fn(),
  lspStop: vi.fn(),
}));
vi.mock("../../platform/dialogs", () => ({ ask: vi.fn(), openPath: vi.fn(), savePath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const projectA: ProjectInfo = { root: "/a", game_code: "CPUD", family: "Pt", language: "de", header_count: 3 };
const projectB: ProjectInfo = { root: "/b", game_code: "CPUD", family: "Pt", language: "de", header_count: 3 };
const MISSING = { code: "tool_missing", message: "rotom-lsp fehlt — Completion und Diagnosen aus", asset: null };
const ADOPTED: ScriptsStatusDto = { adopted: true, file_count: 100 };
const SCRIPT = "script Main #1:\n\tEnd\n";

describe("ScriptPanel", () => {
  let container: HTMLElement;
  let root: Root;
  let bus: EditBus;
  let problems: ProblemsStore;
  let api: ScriptsApi;
  let events: EditEvent[];
  const onError = vi.fn();

  function Host({ project, depth = "bauen" }: { project: ProjectInfo | null; depth?: Depth }) {
    const { api: scripts } = useScripts({ info: project, bus, problems, onError });
    api = scripts;
    return <ScriptPanel context={{ project, scripts, problems, bus, depth, onError }} />;
  }
  const render = (project: ProjectInfo | null = projectA, depth: Depth = "bauen") => act(async () => root.render(<Host project={project} depth={depth} />));
  const settle = () => act(async () => {});

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    bus = createEditBus();
    events = [];
    bus.subscribe(() => true, (e) => events.push(e));
    problems = new ProblemsStore();
    onError.mockReset();
    vi.mocked(scriptsStatus).mockReset().mockResolvedValue(ADOPTED);
    vi.mocked(adoptScripts).mockReset();
    vi.mocked(readScript).mockReset().mockImplementation(async (file) => ({ kind: "script", text: file === 2 ? "script Other #1:\n" : SCRIPT }));
    vi.mocked(saveScript).mockReset().mockResolvedValue({ problems: [], touched_archives: [] });
    vi.mocked(saveLevelScript).mockReset().mockResolvedValue({ problems: [], touched_archives: [] });
    vi.mocked(addScript).mockReset();
    vi.mocked(lspStart).mockReset().mockRejectedValue(MISSING);
    vi.mocked(lspSend).mockReset().mockResolvedValue(undefined);
    vi.mocked(lspStop).mockReset().mockResolvedValue(undefined);
    vi.mocked(ask).mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.body.querySelectorAll("[role=menu]").forEach((m) => m.remove());
  });

  const view = () => EditorView.findFromDOM(container.querySelector(".cm-editor") as HTMLElement)!;
  const typeAt = (pos: number, text: string) => act(() => view().dispatch({ changes: { from: pos, insert: text }, userEvent: "input.type" }));
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const tab = (file: number) => container.querySelector<HTMLButtonElement>(`[data-script="${file}"]`);
  /** Whether the document's tab shows the unsaved dot. */
  const unsaved = (file: number) => tab(file)?.parentElement?.querySelector('[aria-label="ungespeichert"]') != null;
  const title = () => container.querySelector("h2")?.textContent;
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  const open = (file: number, line?: number) => act(async () => api.open(file, line));
  const lspBadge = () => container.querySelector("[data-lsp] > span");

  it("shows_the_adoption_box_until_adopted_then_the_report_and_then_the_editor", async () => {
    vi.mocked(scriptsStatus).mockResolvedValueOnce({ adopted: false, file_count: 12 });
    vi.mocked(adoptScripts).mockImplementation(async (onProgress) => {
      onProgress({ done: 6, total: 12 });
      return { decompiled: 11, skipped_empty: 1, failed: [] };
    });
    await render();
    await settle();
    expect(container.textContent).toContain("Scripts übernehmen");
    expect(container.textContent).toContain("alle 12 Scriptdateien");
    expect(container.querySelector(".cm-editor")).toBeNull();
    vi.mocked(scriptsStatus).mockResolvedValue(ADOPTED);
    click(button("Übernehmen"));
    await settle();
    expect(adoptScripts).toHaveBeenCalledTimes(1);
    // Adopted, but the report is still being read.
    expect(api.status?.adopted).toBe(true);
    expect(container.textContent).toContain("11 Scriptdateien dekompiliert, 1 leere übersprungen.");
    click(button("Weiter"));
    expect(container.textContent).not.toContain("Scripts übernehmen");
    expect(container.textContent).toContain("Kein Script geöffnet");
    // The language server starts once the project is adopted, not before.
    expect(lspStart).toHaveBeenCalledTimes(1);
  });

  it("does_not_start_the_language_server_for_a_project_that_is_not_adopted", async () => {
    vi.mocked(scriptsStatus).mockResolvedValue({ adopted: false, file_count: 1 });
    await render();
    await settle();
    expect(lspStart).not.toHaveBeenCalled();
  });

  it("opens_a_file_into_the_editor_with_its_text_and_a_title", async () => {
    await render();
    await settle();
    await open(42);
    expect(view().state.doc.toString()).toBe(SCRIPT);
    expect(title()).toBe("Scriptdatei 0042");
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
    expect(readScript).toHaveBeenCalledWith(42);
  });

  it("shows_the_open_documents_as_tabs_of_one_tab_list_with_the_shown_one_selected", async () => {
    await render();
    await settle();
    await open(1);
    await open(2);
    const list = container.querySelector('[role="tablist"]');
    expect(list?.getAttribute("aria-label")).toBe("Offene Scripts");
    expect([...list!.querySelectorAll('[role="tab"]')].map((t) => t.textContent)).toEqual(["Scriptdatei 0001", "Scriptdatei 0002"]);
    expect(tab(2)?.getAttribute("role")).toBe("tab");
    expect(tab(2)?.getAttribute("aria-selected")).toBe("true");
    expect(tab(1)?.getAttribute("aria-selected")).toBe("false");
    // The editor is the content of the selected tab.
    expect(container.querySelector('[role="tabpanel"] .cm-editor')).not.toBeNull();
  });

  it("says_how_a_script_opens_while_none_is_open", async () => {
    await render();
    await settle();
    expect(container.querySelector('[role="tablist"]')).toBeNull();
    expect(container.textContent).toContain("Kein Script geöffnet.");
    expect(container.textContent).toContain("Eine Scriptdatei öffnet sich über den Header, ein Event oder ein Problem.");
  });

  it("shows_the_file_path_under_the_title_only_in_tuefteln", async () => {
    await render(projectA, "bauen");
    await settle();
    await open(42);
    expect(title()).toBe("Scriptdatei 0042");
    expect(container.textContent).not.toContain("expanded/scripts/0042.rotom");
    await render(projectA, "tuefteln");
    expect(title()).toBe("Scriptdatei 0042");
    expect(container.querySelector("header")?.textContent).toContain("expanded/scripts/0042.rotom");
  });

  it("opens_a_level_script_as_json_and_saves_the_parsed_document", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { on_load: [1, 2] } });
    await render();
    await settle();
    await open(7);
    expect(title()).toBe("Level-Script 0007 (JSON)");
    expect(JSON.parse(view().state.doc.toString())).toEqual({ on_load: [1, 2] });
    typeAt(view().state.doc.length, " ");
    click(button("Speichern"));
    await settle();
    expect(saveLevelScript).toHaveBeenCalledWith(7, { on_load: [1, 2] });
    expect(saveScript).not.toHaveBeenCalled();
    expect(button("Script anfügen")?.disabled).toBe(true);
  });

  it("refuses_to_save_a_level_script_that_is_not_valid_json_and_says_so", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: {} });
    await render();
    await settle();
    await open(7);
    typeAt(1, "x");
    click(button("Speichern"));
    await settle();
    expect(saveLevelScript).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError.mock.calls[0]![0]).toMatchObject({ code: "error", message: expect.stringContaining("Level-Script 0007: ungültiges JSON") });
    expect(unsaved(7)).toBe(true);
  });

  it("marks_a_document_with_a_dot_while_it_differs_from_the_disk_and_clears_it_on_save", async () => {
    await render();
    await settle();
    await open(42);
    expect(unsaved(42)).toBe(false);
    typeAt(0, "// ");
    expect(unsaved(42)).toBe(true);
    // The name stays the name; the dot says it.
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledWith(42, `// ${SCRIPT}`);
    expect(unsaved(42)).toBe(false);
  });

  it("tells_the_dock_tab_how_many_documents_are_unsaved", async () => {
    await render();
    await settle();
    await open(1);
    await open(2);
    expect(scriptDirtyStore.get()).toBe(0);
    typeAt(0, "B:");
    expect(scriptDirtyStore.get()).toBe(1);
    click(tab(1));
    await settle();
    typeAt(0, "A:");
    expect(scriptDirtyStore.get()).toBe(2);
    click(button("Speichern"));
    await settle();
    expect(scriptDirtyStore.get()).toBe(1);
    // Gone with the panel.
    act(() => root.unmount());
    expect(scriptDirtyStore.get()).toBe(0);
    root = createRoot(container);
  });

  it("saves_with_the_save_button_and_puts_the_compile_problems_of_the_file_into_the_store", async () => {
    const file = "expanded/scripts/0042.rotom";
    vi.mocked(saveScript).mockResolvedValue({
      problems: [
        { severity: "error", file, line: 2, col: 2, message: "unbekannter Befehl", source: "compile" },
        { severity: "warning", file: "expanded/scripts/0043.rotom", line: 1, col: 1, message: "ungenutzt", source: "compile" },
      ],
      touched_archives: [199, 200],
    });
    await render();
    await settle();
    await open(42);
    typeAt(0, "x");
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledTimes(1);
    expect(problems.list().map((p) => [p.file, p.line, p.message, p.source])).toEqual([
      [file, 2, "unbekannter Befehl", "compile"],
      ["expanded/scripts/0043.rotom", 1, "ungenutzt", "compile"],
    ]);
    // Archives the compile appended to: the text panel reads them again.
    expect(events.filter((e) => e.topic === "textData")).toEqual([
      { topic: "textData", archive: 199 },
      { topic: "textData", archive: 200 },
    ]);
    // A later save without problems clears the file's problems and leaves the other file's alone.
    vi.mocked(saveScript).mockResolvedValue({ problems: [], touched_archives: [] });
    typeAt(0, "y");
    click(button("Speichern"));
    await settle();
    expect(problems.list().map((p) => p.file)).toEqual(["expanded/scripts/0043.rotom"]);
  });

  it("editor_saves_without_lsp", async () => {
    vi.mocked(lspStart).mockRejectedValue(MISSING);
    await render();
    await settle();
    await open(42);
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("missing");
    // The badge says the short state; the backend's whole note is its tooltip.
    expect(lspBadge()?.textContent).toBe("rotom-lsp fehlt");
    expect(lspBadge()?.getAttribute("title")).toBe("rotom-lsp fehlt — Completion und Diagnosen aus");
    typeAt(0, "// ");
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledWith(42, `// ${SCRIPT}`);
    expect(onError).not.toHaveBeenCalled();
    expect(unsaved(42)).toBe(false);
    // Without a client, the outline has nothing to ask.
    expect(button("Gliederung")?.disabled).toBe(true);
  });

  it("keeps_each_documents_text_when_switching_between_two", async () => {
    await render();
    await settle();
    await open(1);
    typeAt(0, "A:");
    await open(2);
    expect(view().state.doc.toString()).toBe("script Other #1:\n");
    typeAt(0, "B:");
    click(tab(1));
    await settle();
    expect(view().state.doc.toString()).toBe(`A:${SCRIPT}`);
    click(tab(2));
    await settle();
    expect(view().state.doc.toString()).toBe("B:script Other #1:\n");
    expect([unsaved(1), unsaved(2)]).toEqual([true, true]);
  });

  it("moves_the_cursor_to_the_line_when_a_file_is_opened_at_one", async () => {
    await render();
    await settle();
    await open(42, 2);
    expect(view().state.selection.main.head).toBe("script Main #1:\n".length);
    await open(42, 1);
    expect(view().state.selection.main.head).toBe(0);
  });

  it("normalises_crlf_text_from_disk_so_it_is_not_dirty", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script A #1:\r\n\tEnd\r\n" });
    await render();
    await settle();
    await open(1);
    expect(api.documents.get(1)?.text).toBe("script A #1:\n\tEnd\n");
    expect(api.documents.dirty()).toEqual([]);
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledWith(1, "script A #1:\n\tEnd\n");
  });

  it("asks_before_it_closes_a_dirty_document_and_keeps_it_when_the_answer_is_no", async () => {
    await render();
    await settle();
    await open(42);
    click(container.querySelector('[aria-label="Scriptdatei 0042 schließen"]'));
    await settle();
    // A clean document closes without asking.
    expect(ask).not.toHaveBeenCalled();
    expect(tab(42)).toBeNull();
    await open(42);
    typeAt(0, "x");
    vi.mocked(ask).mockResolvedValueOnce(false);
    click(container.querySelector('[aria-label="Scriptdatei 0042 schließen"]'));
    await settle();
    expect(ask).toHaveBeenCalledWith("Änderungen an Scriptdatei 0042 verwerfen?", { kind: "warning" });
    expect(tab(42)).not.toBeNull();
    vi.mocked(ask).mockResolvedValueOnce(true);
    click(container.querySelector('[aria-label="Scriptdatei 0042 schließen"]'));
    await settle();
    expect(tab(42)).toBeNull();
    expect(container.querySelector(".cm-editor")).toBeNull();
    // Opened again, it shows the disk text and not the discarded edit.
    await open(42);
    expect(view().state.doc.toString()).toBe(SCRIPT);
  });

  it("shows_the_diagnostics_of_the_shown_document_as_live_problems", async () => {
    await render();
    await settle();
    await open(42);
    act(() => view().dispatch(setDiagnostics(view().state, [{ from: 17, to: 20, severity: "error", message: "bad end" }, { from: 0, to: 6, severity: "info", message: "hint" }])));
    expect(problems.list()).toEqual([{ severity: "error", file: "expanded/scripts/0042.rotom", line: 2, col: 2, message: "bad end", source: "live" }]);
    // Another document replaces them; closing the last one clears them.
    await open(2);
    expect(problems.list()).toEqual([]);
  });

  it("appends_a_script_through_the_backend_and_shows_the_new_text_with_the_cursor_at_its_end", async () => {
    vi.mocked(addScript).mockResolvedValue(2);
    await render();
    await settle();
    await open(42);
    typeAt(0, "// ");
    vi.mocked(readScript).mockResolvedValue({ kind: "script", text: `// ${SCRIPT}\nscript Script2 #2:\n` });
    click(button("Script anfügen"));
    await settle();
    // The unsaved edit was saved first: the backend appends to what is on disk.
    expect(vi.mocked(saveScript).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(addScript).mock.invocationCallOrder[0]!);
    expect(addScript).toHaveBeenCalledWith(42);
    expect(view().state.doc.toString()).toBe(`// ${SCRIPT}\nscript Script2 #2:\n`);
    expect(view().state.selection.main.head).toBe(view().state.doc.length);
    expect(api.documents.dirty()).toEqual([]);
    expect(events).toContainEqual({ topic: "textData", archive: null });
  });

  it("resolves_add_to_the_new_scripts_number_and_to_null_when_it_failed", async () => {
    vi.mocked(addScript).mockResolvedValueOnce(5).mockRejectedValueOnce({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    await render();
    await settle();
    let added: number | null | undefined;
    await act(async () => void (added = await api.add(42)));
    expect(added).toBe(5);
    await act(async () => void (added = await api.add(42)));
    expect(added).toBeNull();
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
  });

  describe("a level script saved by the form", () => {
    const LEVEL = "expanded/scripts/0007.json";
    const doc = { header_entries: [{ type: "on_load", script_id: 3 }], var_conditions: [] };
    const dto = (over: Partial<SaveScriptDto> = {}): SaveScriptDto => ({ problems: [], touched_archives: [], ...over });

    it("puts_its_problems_into_the_store_per_file_and_tells_the_text_panel_about_touched_archives", async () => {
      await render();
      await settle();
      act(() => api.onLevelScriptSaved(projectA, 7, doc, dto({ problems: [{ severity: "error", file: LEVEL, line: 1, col: 1, message: "var darf nicht 0 sein", source: "compile" }], touched_archives: [199] })));
      expect(problems.list().map((p) => [p.file, p.message, p.source])).toEqual([[LEVEL, "var darf nicht 0 sein", "compile"]]);
      expect(events.filter((e) => e.topic === "textData")).toEqual([{ topic: "textData", archive: 199 }]);
      // A later clean save clears the file.
      act(() => api.onLevelScriptSaved(projectA, 7, doc, dto()));
      expect(problems.list()).toEqual([]);
    });

    it("shows_the_new_json_in_a_clean_open_document_and_leaves_a_dirty_one_alone", async () => {
      vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [], var_conditions: [] } });
      await render();
      await settle();
      await open(7);
      act(() => api.onLevelScriptSaved(projectA, 7, doc, dto()));
      expect(JSON.parse(view().state.doc.toString())).toEqual(doc);
      expect(api.documents.dirty()).toEqual([]);
      // Text typed in the editor is the user's: the form's save does not replace it.
      typeAt(0, " ");
      const typed = view().state.doc.toString();
      act(() => api.onLevelScriptSaved(projectA, 7, { header_entries: [], var_conditions: [] }, dto()));
      expect(view().state.doc.toString()).toBe(typed);
    });

    it("is_ignored_once_another_project_is_open", async () => {
      await render(projectA);
      await settle();
      await render(projectB);
      await settle();
      act(() => api.onLevelScriptSaved(projectA, 7, doc, dto({ problems: [{ severity: "error", file: LEVEL, line: 1, col: 1, message: "alt", source: "compile" }], touched_archives: [1] })));
      expect(problems.list()).toEqual([]);
      expect(events.filter((e) => e.topic === "textData")).toEqual([]);
    });
  });

  it("reports_a_binary_file_and_opens_nothing", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "binary", size: 120 });
    await render();
    await settle();
    await open(5);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError.mock.calls[0]![0]).toMatchObject({ message: "Scriptdatei 0005 ist binär (120 Byte) und lässt sich nicht bearbeiten" });
    expect(tab(5)).toBeNull();
  });

  it("connects_the_language_server_when_there_is_one_and_ends_it_with_the_project", async () => {
    vi.mocked(lspStart).mockResolvedValue(undefined);
    await render(projectA);
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("running");
    expect(lspStart).toHaveBeenCalledTimes(1);
    // Another project: the old server is stopped and a fresh one started on a new channel.
    await render(projectB);
    await settle();
    await settle();
    expect(lspStop).toHaveBeenCalledTimes(1);
    expect(lspStart).toHaveBeenCalledTimes(2);
    expect(vi.mocked(lspStop).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(lspStart).mock.invocationCallOrder[1]!);
    // The documents of the old project are gone.
    expect(api.documents.list()).toEqual([]);
  });

  /** A language server that answers `initialize` and the given methods; returns what the client sent. */
  function answeringServer(answers: Record<string, (params: any) => unknown> = {}) {
    const sent: { id?: number; method: string; params?: any }[] = [];
    let push: (body: string) => void = () => {};
    vi.mocked(lspStart).mockImplementation(async (onMessage) => void (push = onMessage));
    vi.mocked(lspSend).mockImplementation(async (body) => {
      const message = JSON.parse(body);
      sent.push(message);
      if (message.method === "initialize") queueMicrotask(() => push(JSON.stringify({ jsonrpc: "2.0", id: message.id, result: { capabilities: { documentSymbolProvider: true } } })));
      const answer = answers[message.method];
      if (answer && message.id !== undefined) queueMicrotask(() => push(JSON.stringify({ jsonrpc: "2.0", id: message.id, result: answer(message.params) })));
    });
    return sent;
  }

  it("tells_a_running_language_server_which_files_changed_with_file_uris", async () => {
    const sent = answeringServer();
    await render();
    await settle();
    await settle();
    act(() =>
      bus.emit({
        topic: "lspFilesChanged",
        changes: [
          { path: "expanded/textArchives/0005.json", type: 2 },
          { path: "rotom.toml", type: 1 },
        ],
      }),
    );
    await settle();
    await settle();
    const notice = sent.filter((m) => m.method === "workspace/didChangeWatchedFiles");
    expect(notice).toHaveLength(1);
    expect(notice[0]!.params).toEqual({
      changes: [
        { uri: "file:///a/expanded/textArchives/0005.json", type: 2 },
        { uri: "file:///a/rotom.toml", type: 1 },
      ],
    });
  });

  it("sends_no_file_notice_when_the_language_server_is_not_running", async () => {
    vi.mocked(lspStart).mockRejectedValue(MISSING);
    await render();
    await settle();
    act(() => bus.emit({ topic: "lspFilesChanged", changes: [{ path: "expanded/textArchives/0005.json", type: 2 }] }));
    await settle();
    expect(lspSend).not.toHaveBeenCalled();
  });

  it("announces_rotom_toml_as_created_after_an_adoption", async () => {
    vi.mocked(scriptsStatus).mockResolvedValueOnce({ adopted: false, file_count: 2 });
    vi.mocked(adoptScripts).mockResolvedValue({ decompiled: 2, skipped_empty: 0, failed: [] });
    await render();
    await settle();
    click(button("Übernehmen"));
    await settle();
    expect(events).toContainEqual({ topic: "lspFilesChanged", changes: [{ path: "rotom.toml", type: 1 }] });
  });

  it("drops_a_save_that_finishes_after_another_project_was_opened_and_does_not_let_the_next_project_join_it", async () => {
    let finish!: (dto: { problems: any[]; touched_archives: number[] }) => void;
    vi.mocked(saveScript).mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
    await render(projectA);
    await settle();
    await open(42);
    api.documents.get(42)!.text += "x";
    const inFlight = api.save(42);
    await render(projectB);
    await settle();
    expect(api.documents.list()).toEqual([]);
    // The old project's save ends now: its compile rows and archives are nobody's in the new project.
    finish({ problems: [{ severity: "error", file: "expanded/scripts/0042.rotom", line: 1, col: 1, message: "alt", source: "compile" }], touched_archives: [199] });
    await act(async () => void (await inFlight));
    expect(problems.list()).toEqual([]);
    expect(events.filter((e) => e.topic === "textData")).toEqual([]);
    // The same file number in the new project saves on its own, it does not join the old save.
    await open(42);
    api.documents.get(42)!.text += "y";
    await act(async () => api.save(42));
    expect(saveScript).toHaveBeenCalledTimes(2);
    expect(saveScript).toHaveBeenLastCalledWith(42, `${SCRIPT}y`);
    expect(api.documents.dirty()).toEqual([]);
  });

  it("a_save_that_joins_one_in_flight_saves_the_text_typed_meanwhile_right_after_it", async () => {
    let finish!: (dto: { problems: any[]; touched_archives: number[] }) => void;
    vi.mocked(saveScript).mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
    await render();
    await settle();
    await open(42);
    const doc = api.documents.get(42)!;
    doc.text += "a";
    const first = api.save(42);
    doc.text += "b";
    const joined = api.save(42);
    expect(saveScript).toHaveBeenCalledTimes(1);
    expect(saveScript).toHaveBeenCalledWith(42, `${SCRIPT}a`);
    await act(async () => {
      finish({ problems: [], touched_archives: [] });
      await Promise.all([first, joined]);
    });
    expect(saveScript).toHaveBeenCalledTimes(2);
    expect(saveScript).toHaveBeenLastCalledWith(42, `${SCRIPT}ab`);
    expect(api.documents.dirty()).toEqual([]);
    // Nothing typed meanwhile: a joined save does not write again.
    let done!: (dto: { problems: any[]; touched_archives: number[] }) => void;
    vi.mocked(saveScript).mockImplementationOnce(() => new Promise((resolve) => (done = resolve)));
    doc.text += "c";
    const third = api.save(42);
    const joinedClean = api.save(42);
    await act(async () => {
      done({ problems: [], touched_archives: [] });
      await Promise.all([third, joinedClean]);
    });
    expect(saveScript).toHaveBeenCalledTimes(3);
  });

  it("starts_one_language_server_under_strict_mode_and_does_not_stop_it", async () => {
    vi.mocked(lspStart).mockResolvedValue(undefined);
    await act(async () =>
      root.render(
        <StrictMode>
          <Host project={projectA} />
        </StrictMode>,
      ),
    );
    await settle();
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("running");
    expect(lspStart).toHaveBeenCalledTimes(1);
    expect(lspStop).not.toHaveBeenCalled();
  });

  it("shows_the_language_server_state_as_a_badge", async () => {
    vi.mocked(lspStart).mockResolvedValue(undefined);
    await render();
    await settle();
    const badge = container.querySelector("[data-lsp]");
    expect(badge?.getAttribute("data-lsp")).toBe("running");
    expect(badge?.textContent).toBe("rotom-lsp läuft");
    expect(button("Neu starten")).toBeUndefined();
  });

  it("loads_the_outline_when_its_menu_opens_and_jumps_to_the_picked_script", async () => {
    const sent = answeringServer({
      "textDocument/documentSymbol": () => [
        { name: "Main", detail: "#1", kind: 12, range: { start: { line: 0, character: 0 }, end: { line: 1, character: 4 } }, selectionRange: { start: { line: 0, character: 7 }, end: { line: 0, character: 11 } } },
        { name: "Other", kind: 12, range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } } },
      ],
    });
    await render();
    await settle();
    await settle();
    await open(42, 1);
    await settle();
    // Nothing is asked before the menu opens.
    expect(sent.filter((m) => m.method === "textDocument/documentSymbol")).toHaveLength(0);
    const trigger = button("Gliederung")!;
    expect(trigger.disabled).toBe(false);
    await act(async () => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    await settle();
    await settle();
    expect(sent.filter((m) => m.method === "textDocument/documentSymbol")).toHaveLength(1);
    const items = () => [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
    expect(items().map((i) => i.textContent)).toEqual(["Main #1, Z. 1", "Other, Z. 2"]);
    await act(async () => items()[1]!.click());
    await settle();
    expect(view().state.selection.main.head).toBe("script Main #1:\n".length);
  });

  it("offers_to_start_the_language_server_again_when_it_died", async () => {
    vi.mocked(lspStart).mockRejectedValueOnce({ code: "error", message: "rotom-lsp beendet sich wiederholt", asset: null });
    await render();
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("stopped");
    expect(lspBadge()?.textContent).toBe("rotom-lsp gestoppt");
    expect(lspBadge()?.getAttribute("title")).toBe("rotom-lsp beendet sich wiederholt");
    vi.mocked(lspStart).mockResolvedValue(undefined);
    click(button("Neu starten"));
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("running");
  });

  describe("tool status", () => {
    beforeEach(() => toolStatus.set({ lsp: null }));

    it("writes_the_language_server_state_and_its_restart_into_the_tool_status", async () => {
      expect(toolStatus.get().lsp).toBeNull();
      vi.mocked(lspStart).mockRejectedValueOnce({ code: "error", message: "rotom-lsp beendet sich wiederholt", asset: null });
      await render();
      await settle();
      expect(toolStatus.get().lsp).toMatchObject({ state: "fehler", note: "rotom-lsp beendet sich wiederholt" });
      // Technik's "Neu starten" restarts the panel's server.
      vi.mocked(lspStart).mockResolvedValue(undefined);
      await act(async () => toolStatus.get().lsp?.restart?.());
      await settle();
      expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("running");
      expect(toolStatus.get().lsp?.state).toBe("bereit");
    });

    it("says_missing_for_a_missing_server_and_off_without_adoption_or_once_the_panel_is_gone", async () => {
      await render();
      await settle();
      expect(toolStatus.get().lsp).toMatchObject({ state: "fehlt", note: "rotom-lsp fehlt — Completion und Diagnosen aus" });
      vi.mocked(scriptsStatus).mockResolvedValue({ adopted: false, file_count: 3 });
      await render(projectB);
      await settle();
      // Off for want of the adoption, and Technik says so.
      expect(toolStatus.get().lsp).toMatchObject({ state: "aus", note: "Läuft, sobald die Scripts des Projekts übernommen sind." });
      vi.mocked(scriptsStatus).mockResolvedValue(ADOPTED);
      await render(projectA);
      await settle();
      expect(toolStatus.get().lsp?.state).toBe("fehlt");
      act(() => root.render(<></>));
      // Gone with the panel, scripts adopted: off without a reason that would not hold.
      expect(toolStatus.get().lsp?.state).toBe("aus");
      expect(toolStatus.get().lsp?.note).toBeUndefined();
    });
  });
});
