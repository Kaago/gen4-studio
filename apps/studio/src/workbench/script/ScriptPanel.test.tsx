// @vitest-environment jsdom
import { setDiagnostics } from "@codemirror/lint";
import { EditorView } from "@codemirror/view";
import { ask } from "@tauri-apps/plugin-dialog";
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectInfo, ScriptsStatusDto } from "../../api";
import { addScript, adoptScripts, lspStart, lspStop, readScript, saveLevelScript, saveScript, scriptsStatus } from "../../api";
import { createEditBus } from "../../bus/editBus";
import type { EditBus, EditEvent } from "../../bus/editBus";
import { ProblemsStore } from "../problems/problems";
import { ScriptPanel } from "./ScriptPanel";
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
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

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

  function Host({ project }: { project: ProjectInfo | null }) {
    const { api: scripts } = useScripts({ info: project, bus, problems, onError });
    api = scripts;
    return <ScriptPanel context={{ project, scripts, problems, onError }} />;
  }
  const render = (project: ProjectInfo | null = projectA) => act(async () => root.render(<Host project={project} />));
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
    vi.mocked(lspStop).mockReset().mockResolvedValue(undefined);
    vi.mocked(ask).mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const view = () => EditorView.findFromDOM(container.querySelector(".cm-editor") as HTMLElement)!;
  const typeAt = (pos: number, text: string) => act(() => view().dispatch({ changes: { from: pos, insert: text }, userEvent: "input.type" }));
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const tab = (file: number) => container.querySelector<HTMLButtonElement>(`[data-script="${file}"]`);
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  const open = (file: number, line?: number) => act(async () => api.open(file, line));

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
    expect(container.querySelector("strong")?.textContent).toBe("Scriptdatei 0042");
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
    expect(readScript).toHaveBeenCalledWith(42);
  });

  it("opens_a_level_script_as_json_and_saves_the_parsed_document", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { on_load: [1, 2] } });
    await render();
    await settle();
    await open(7);
    expect(container.querySelector("strong")?.textContent).toBe("Level-Script 0007 (JSON)");
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
    expect(tab(7)?.textContent).toContain("●");
  });

  it("marks_a_document_with_a_dot_while_it_differs_from_the_disk_and_clears_it_on_save", async () => {
    await render();
    await settle();
    await open(42);
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
    typeAt(0, "// ");
    expect(tab(42)?.textContent).toBe("● Scriptdatei 0042");
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledWith(42, `// ${SCRIPT}`);
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
  });

  it("saves_with_cmd_s_in_the_editor_and_puts_the_compile_problems_of_the_file_into_the_store", async () => {
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
    const content = container.querySelector(".cm-content") as HTMLElement;
    act(() => void content.dispatchEvent(new KeyboardEvent("keydown", { key: "s", code: "KeyS", ctrlKey: true, bubbles: true, cancelable: true })));
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
    expect(container.textContent).toContain("rotom-lsp fehlt — Completion und Diagnosen aus");
    typeAt(0, "// ");
    click(button("Speichern"));
    await settle();
    expect(saveScript).toHaveBeenCalledWith(42, `// ${SCRIPT}`);
    expect(onError).not.toHaveBeenCalled();
    expect(tab(42)?.textContent).toBe("Scriptdatei 0042");
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
    expect(tab(1)?.textContent).toBe("● Scriptdatei 0001");
    expect(tab(2)?.textContent).toBe("● Scriptdatei 0002");
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

  it("offers_to_start_the_language_server_again_when_it_died", async () => {
    vi.mocked(lspStart).mockRejectedValueOnce({ code: "error", message: "rotom-lsp beendet sich wiederholt", asset: null });
    await render();
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("stopped");
    expect(container.textContent).toContain("rotom-lsp beendet sich wiederholt");
    vi.mocked(lspStart).mockResolvedValue(undefined);
    click(button("Neu starten"));
    await settle();
    expect(container.querySelector("[data-lsp]")?.getAttribute("data-lsp")).toBe("running");
  });
});
