// @vitest-environment jsdom
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BuildReport, ProblemDto } from "../api";
import { buildRom, dirty, headerPatchStatus, listHeaders, memberCounts, openProject, readScript, save, saveScript, scriptsStatus } from "../api";
import { createEditBus } from "../bus/editBus";
import { ProblemsStore } from "../workbench/problems/problems";
import { NO_SCRIPTS, useScripts } from "../workbench/script/useScripts";
import type { ScriptBridge, ScriptsController } from "../workbench/script/useScripts";
import { useProjectSession } from "./useProjectSession";
import type { ProjectSession } from "./useProjectSession";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  buildRom: vi.fn(),
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
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const problem = (file: string, line: number, message: string, severity: ProblemDto["severity"] = "error", source: ProblemDto["source"] = "compile"): ProblemDto => ({ severity, file, line, col: 1, message, source });
const report = (over: Partial<BuildReport> = {}): BuildReport => ({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], ...over });

describe("useProjectSession with open scripts", () => {
  let container: HTMLElement;
  let root: Root;
  let session: ProjectSession;
  let scripts: ScriptsController;
  let problems: ProblemsStore;

  function Host() {
    const bridge = useRef<ScriptBridge>(NO_SCRIPTS);
    session = useProjectSession({ problems, scripts: bridge });
    scripts = useScripts({ info: session.info, bus: createEditBus(), problems, onError: session.fail });
    bridge.current = scripts.bridge;
    return null;
  }

  /** Opens the files and edits each one, as the editor does: the document text moves away from what is on disk. */
  async function openDirty(...files: number[]) {
    for (const file of files) await act(async () => scripts.api.open(file));
    for (const file of files) scripts.api.documents.get(file)!.text += "// edited\n";
  }
  const build = () => act(async () => session.onBuild());

  beforeEach(async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    problems = new ProblemsStore();
    vi.mocked(saveDialog).mockReset().mockResolvedValue("/out.nds");
    vi.mocked(ask).mockReset().mockResolvedValue(true);
    vi.mocked(open).mockReset();
    vi.mocked(buildRom).mockReset().mockResolvedValue(report());
    vi.mocked(save).mockReset().mockResolvedValue({ written: [], problems: [] });
    vi.mocked(dirty).mockReset().mockResolvedValue([]);
    vi.mocked(openProject).mockReset();
    vi.mocked(listHeaders).mockReset().mockResolvedValue({ headers: [], names_error: null });
    vi.mocked(memberCounts).mockReset().mockResolvedValue({});
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue({ supported: true, applied: true, repairable: false, narc: null });
    vi.mocked(scriptsStatus).mockReset().mockResolvedValue({ adopted: true, file_count: 10 });
    vi.mocked(readScript).mockReset().mockImplementation(async (file) => ({ kind: "script", text: `script F${file} #1:\n` }));
    vi.mocked(saveScript).mockReset().mockResolvedValue({ problems: [], touched_archives: [] });
    await act(async () => root.render(<Host />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("build_saves_open_scripts_first_and_stops_on_failure", async () => {
    await openDirty(3, 4);
    await build();
    expect(vi.mocked(saveScript).mock.calls.map((c) => c[0])).toEqual([3, 4]);
    expect(buildRom).toHaveBeenCalledWith("/out.nds");
    const order = (m: { mock: { invocationCallOrder: number[] } }) => m.mock.invocationCallOrder;
    expect(Math.max(...order(vi.mocked(saveScript)))).toBeLessThan(order(vi.mocked(buildRom))[0]!);
    expect(scripts.api.documents.dirty()).toEqual([]);
    expect(session.message?.text).toContain("ROM gebaut: /out.nds");
    expect(session.busy).toBe(false);

    // The first save fails: the second is not tried, nothing is built, and the error shows.
    vi.mocked(saveScript).mockClear();
    vi.mocked(buildRom).mockClear();
    await openDirty(3, 4);
    vi.mocked(saveScript).mockRejectedValueOnce({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    await build();
    expect(saveScript).toHaveBeenCalledTimes(1);
    expect(buildRom).not.toHaveBeenCalled();
    expect(session.message).toMatchObject({ kind: "error", text: "Textarchiv 0199 erst speichern" });
    expect(session.busy).toBe(false);
    // Both documents are still unsaved.
    expect(scripts.api.documents.dirty()).toEqual([3, 4]);
  });

  it("build_does_not_save_clean_scripts_and_a_cancelled_dialog_saves_nothing", async () => {
    await act(async () => scripts.api.open(3));
    await build();
    expect(saveScript).not.toHaveBeenCalled();
    expect(buildRom).toHaveBeenCalledTimes(1);
    await openDirty(3);
    vi.mocked(saveDialog).mockResolvedValueOnce(null);
    vi.mocked(buildRom).mockClear();
    await build();
    expect(saveScript).not.toHaveBeenCalled();
    expect(buildRom).not.toHaveBeenCalled();
  });

  it("build_puts_compile_problems_of_the_saves_into_the_store_and_still_builds", async () => {
    await openDirty(3);
    vi.mocked(saveScript).mockResolvedValue({ problems: [problem("expanded/scripts/0003.rotom", 2, "unbekannter Befehl", "warning")], touched_archives: [] });
    await build();
    expect(problems.list().map((p) => p.message)).toEqual(["unbekannter Befehl"]);
    expect(buildRom).toHaveBeenCalledTimes(1);
  });

  it("a_compile_error_of_the_build_fills_the_problems_store_and_says_how_many_errors", async () => {
    vi.mocked(buildRom).mockRejectedValue({
      code: "compile",
      message: "2 Fehler in Scripts oder Texten",
      asset: null,
      problems: [
        problem("expanded/scripts/0003.rotom", 2, "e1"),
        problem("expanded/textArchives/0199.json", 1, "e2", "error", "text"),
        problem("expanded/scripts/0003.rotom", 9, "w1", "warning"),
      ],
    });
    await build();
    expect(problems.list().map((p) => [p.message, p.source])).toEqual([
      ["e1", "compile"],
      ["e2", "text"],
      ["w1", "compile"],
    ]);
    expect(session.message).toMatchObject({ kind: "error", text: "Build abgebrochen: 2 Fehler" });
    expect(session.busy).toBe(false);
    expect(session.lastRom).toBeNull();
  });

  it("a_compile_error_without_a_problem_list_still_aborts_with_the_backends_message", async () => {
    vi.mocked(buildRom).mockRejectedValue({ code: "compile", message: "1 Fehler in Scripts oder Texten", asset: null });
    await build();
    expect(session.message).toMatchObject({ kind: "error", text: "Build abgebrochen: 1 Fehler in Scripts oder Texten" });
    expect(problems.list()).toEqual([]);
  });

  it("another_error_of_the_build_is_shown_as_it_is", async () => {
    vi.mocked(buildRom).mockRejectedValue({ code: "error", message: "dsrom fehlt", asset: null });
    await build();
    expect(session.message).toMatchObject({ kind: "error", text: "dsrom fehlt" });
  });

  it("keeps_the_warnings_of_a_successful_build_in_the_problems_store", async () => {
    vi.mocked(buildRom).mockResolvedValue(report({ problems: [problem("expanded/textArchives/0007.json", 1, "zu breit", "warning", "text")] }));
    await build();
    expect(problems.list().map((p) => [p.message, p.source])).toEqual([["zu breit", "text"]]);
  });

  describe("with a project open", () => {
    beforeEach(async () => {
      vi.mocked(open).mockResolvedValue("/p");
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 0 });
      await act(async () => session.onOpen(() => {}));
    });

    it("save_writes_the_store_and_then_the_open_scripts_and_counts_both", async () => {
      vi.mocked(save).mockResolvedValue({ written: [{ kind: "TextArchive", index: 5 }, { kind: "Map", index: 1 }], problems: [problem("expanded/textArchives/0005.json", 1, "zu breit", "warning", "text")] });
      await openDirty(3);
      await act(async () => session.onSave());
      expect(saveScript).toHaveBeenCalledWith(3, "script F3 #1:\n// edited\n");
      expect(vi.mocked(save).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(saveScript).mock.invocationCallOrder[0]!);
      expect(session.message).toMatchObject({ kind: "info", text: "3 Datei(en) gespeichert" });
      expect(problems.list().map((p) => p.message)).toEqual(["zu breit"]);
      // The next save of archive 5 without warnings clears them.
      vi.mocked(save).mockResolvedValue({ written: [{ kind: "TextArchive", index: 5 }], problems: [] });
      await act(async () => session.onSave());
      expect(problems.list()).toEqual([]);
    });

    it("opening_another_project_asks_when_only_a_script_is_unsaved", async () => {
      await openDirty(3);
      vi.mocked(ask).mockResolvedValueOnce(false);
      vi.mocked(openProject).mockClear();
      await act(async () => session.onOpen(() => {}));
      expect(ask).toHaveBeenCalledWith("Ungespeicherte Änderungen verwerfen?", { kind: "warning" });
      expect(openProject).not.toHaveBeenCalled();
    });
  });
});
