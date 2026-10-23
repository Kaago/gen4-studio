// @vitest-environment jsdom
import { EditorView } from "@codemirror/view";
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dirty, headerPatchStatus, listHeaders, memberCounts, openProject, readScript, save, saveScript, scriptsStatus } from "../api";
import { createEditBus } from "../bus/editBus";
import { ProblemsStore } from "../workbench/problems/problems";
import { ScriptEditor } from "../workbench/script/ScriptEditor";
import { NO_SCRIPTS, useScripts } from "../workbench/script/useScripts";
import type { ScriptBridge, ScriptsController } from "../workbench/script/useScripts";
import { useProjectSession } from "./useProjectSession";
import type { ProjectSession } from "./useProjectSession";
import { useShortcuts } from "./useShortcuts";
import type { InspectorModel } from "../workbench/Inspector";
import { openPath } from "../platform/dialogs";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  save: vi.fn(),
  dirty: vi.fn(),
  openProject: vi.fn(),
  listHeaders: vi.fn(),
  memberCounts: vi.fn(),
  headerPatchStatus: vi.fn(),
  scriptsStatus: vi.fn(),
  readScript: vi.fn(),
  saveScript: vi.fn(),
}));
vi.mock("../platform/dialogs", () => ({ ask: vi.fn(), openPath: vi.fn(), savePath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Cmd+S is one window-level shortcut: the same keystroke with the cursor inside the script editor saves the store, then the script, once each. */
describe("Cmd+S inside the script editor", () => {
  let container: HTMLElement;
  let root: Root;
  let session: ProjectSession;
  let scripts: ScriptsController;
  const pinned = { current: { kind: "none" } as InspectorModel };

  function Host() {
    const bridge = useRef<ScriptBridge>(NO_SCRIPTS);
    session = useProjectSession({ bus, problems, scripts: bridge });
    scripts = useScripts({ info: session.info, bus, problems, onError: session.fail });
    bridge.current = scripts.bridge;
    useShortcuts(pinned, {
      onSave: session.onSave,
      step: async () => {},
      onDeleteBuilding: () => {},
      onDeleteEvent: () => {},
      unpinSelection: () => {},
    });
    const doc = scripts.api.documents.get(3);
    return doc ? <ScriptEditor doc={doc} language="rotom" lsp={null} uri="file:///p/expanded/scripts/0003.rotom" onChange={() => scripts.api.onChange(3)} onDiagnostics={() => {}} theme="light" /> : null;
  }
  const bus = createEditBus();
  const problems = new ProblemsStore();

  beforeEach(async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(openPath).mockReset().mockResolvedValue("/p");
    vi.mocked(save).mockReset().mockResolvedValue({ written: [{ kind: "TextArchive", index: 5 }], problems: [] });
    vi.mocked(dirty).mockReset().mockResolvedValue([]);
    vi.mocked(openProject).mockReset().mockResolvedValue({ root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 0 });
    vi.mocked(listHeaders).mockReset().mockResolvedValue({ headers: [], names_error: null });
    vi.mocked(memberCounts).mockReset().mockResolvedValue({});
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue({ supported: true, applied: true, repairable: false, narc: null });
    vi.mocked(scriptsStatus).mockReset().mockResolvedValue({ adopted: true, file_count: 10 });
    vi.mocked(readScript).mockReset().mockImplementation(async (file) => ({ kind: "script", text: `script F${file} #1:\n` }));
    vi.mocked(saveScript).mockReset().mockResolvedValue({ problems: [], touched_archives: [] });
    await act(async () => root.render(<Host />));
    await act(async () => session.onOpen(() => {}));
    await act(async () => scripts.api.open(3));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("saves_the_store_and_then_the_script_exactly_once", async () => {
    const view = EditorView.findFromDOM(container.querySelector(".cm-editor") as HTMLElement)!;
    act(() => view.dispatch({ changes: { from: 0, insert: "// edited\n" }, userEvent: "input.type" }));
    expect(scripts.api.documents.dirty()).toEqual([3]);

    const content = container.querySelector(".cm-content") as HTMLElement;
    const event = new KeyboardEvent("keydown", { key: "s", code: "KeyS", ctrlKey: true, bubbles: true, cancelable: true });
    await act(async () => void content.dispatchEvent(event));

    // The window handler took it (and kept the browser's own "save page" away).
    expect(event.defaultPrevented).toBe(true);
    expect(save).toHaveBeenCalledTimes(1);
    expect(saveScript).toHaveBeenCalledTimes(1);
    expect(saveScript).toHaveBeenCalledWith(3, "// edited\nscript F3 #1:\n");
    expect(vi.mocked(save).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(saveScript).mock.invocationCallOrder[0]!);
    expect(session.message).toMatchObject({ kind: "info", text: "2 Datei(en) gespeichert" });
    expect(scripts.api.documents.dirty()).toEqual([]);
  });
});
