// @vitest-environment jsdom
import { ask, openPath, savePath } from "../platform/dialogs";
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BuildReport, ProblemDto } from "../api";
import { buildRom, dirty, headerPatchStatus, listHeaders, memberCounts, openProject, readScript, runRom, save, saveScript, scriptsStatus } from "../api";
import { createEditBus } from "../bus/editBus";
import type { EditBus, EditEvent } from "../bus/editBus";
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
  runRom: vi.fn(),
}));
vi.mock("../platform/dialogs", () => ({ ask: vi.fn(), openPath: vi.fn(), savePath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const problem = (file: string, line: number, message: string, severity: ProblemDto["severity"] = "error", source: ProblemDto["source"] = "compile"): ProblemDto => ({ severity, file, line, col: 1, message, source });
const report = (over: Partial<BuildReport> = {}): BuildReport => ({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], ...over });

describe("useProjectSession with open scripts", () => {
  let container: HTMLElement;
  let root: Root;
  let session: ProjectSession;
  let scripts: ScriptsController;
  let problems: ProblemsStore;
  let bus: EditBus;
  let events: EditEvent[];
  let showProblems: ReturnType<typeof vi.fn<() => void>>;

  function Host() {
    const bridge = useRef<ScriptBridge>(NO_SCRIPTS);
    session = useProjectSession({ bus, problems, scripts: bridge, onShowProblems: showProblems });
    scripts = useScripts({ info: session.info, bus, problems, onError: session.fail });
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
    bus = createEditBus();
    events = [];
    showProblems = vi.fn<() => void>();
    bus.subscribe(() => true, (e) => events.push(e));
    vi.mocked(savePath).mockReset().mockResolvedValue("/out.nds");
    vi.mocked(ask).mockReset().mockResolvedValue(true);
    vi.mocked(openPath).mockReset();
    vi.mocked(buildRom).mockReset().mockResolvedValue(report());
    vi.mocked(save).mockReset().mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
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

  it("keeps_the_list_of_dirty_assets_next_to_the_count_and_the_same_list_while_it_does_not_change", async () => {
    expect(session.dirtyAssets).toEqual([]);
    vi.mocked(dirty).mockResolvedValue([{ kind: "Event", index: 8 }]);
    await act(async () => session.refreshDirty());
    expect(session.dirtyCount).toBe(1);
    expect(session.dirtyAssets).toEqual([{ kind: "Event", index: 8 }]);
    // Another read with the same assets (a fresh array from the backend) leaves the list as it was.
    const before = session.dirtyAssets;
    vi.mocked(dirty).mockResolvedValue([{ kind: "Event", index: 8 }]);
    await act(async () => session.refreshDirty());
    expect(session.dirtyAssets).toBe(before);
    // A save empties it.
    vi.mocked(dirty).mockResolvedValue([]);
    await act(async () => session.refreshDirty());
    expect(session.dirtyAssets).toEqual([]);
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

  it("build_saves_the_store_before_the_open_scripts_so_a_dirty_text_archive_does_not_refuse_them", async () => {
    // The backend refuses a script save while a text archive is unsaved: the store has to go first.
    let archiveSaved = false;
    vi.mocked(save).mockImplementation(async () => {
      archiveSaved = true;
      return { compiled: [], touched_archives: [], written: [{ kind: "TextArchive", index: 5 }, { kind: "Map", index: 1 }], problems: [problem("expanded/textArchives/0005.json", 1, "zu breit", "warning", "text")] };
    });
    vi.mocked(saveScript).mockImplementation(async () => {
      if (!archiveSaved) throw { code: "error", message: "Textarchiv 0005 erst speichern", asset: null };
      return { problems: [], touched_archives: [] };
    });
    await openDirty(3);
    await build();
    expect(session.message).toMatchObject({ kind: "info", text: "ROM gebaut: /out.nds (2 Datei(en) vorher gespeichert)" });
    const first = (m: { mock: { invocationCallOrder: number[] } }) => m.mock.invocationCallOrder[0]!;
    expect(first(vi.mocked(save))).toBeLessThan(first(vi.mocked(saveScript)));
    expect(first(vi.mocked(saveScript))).toBeLessThan(first(vi.mocked(buildRom)));
    expect(problems.list().map((p) => p.message)).toEqual(["zu breit"]);
    // The language server is told which archive changed.
    expect(events).toContainEqual({ topic: "lspFilesChanged", changes: [{ path: "expanded/textArchives/0005.json", type: 2 }] });
    // A failing store save builds nothing and keeps the scripts unsaved.
    vi.mocked(save).mockRejectedValue({ code: "error", message: "Platte voll", asset: null });
    vi.mocked(buildRom).mockClear();
    vi.mocked(saveScript).mockClear();
    await openDirty(3);
    await build();
    expect(session.message).toMatchObject({ kind: "error", text: "Platte voll" });
    expect(buildRom).not.toHaveBeenCalled();
    expect(saveScript).not.toHaveBeenCalled();
  });

  it("build_does_not_save_clean_scripts_and_a_cancelled_dialog_saves_nothing", async () => {
    await act(async () => scripts.api.open(3));
    await build();
    expect(saveScript).not.toHaveBeenCalled();
    // Nothing of the store is saved by the editor side: the build does that itself.
    expect(save).not.toHaveBeenCalled();
    expect(buildRom).toHaveBeenCalledTimes(1);
    await openDirty(3);
    vi.mocked(savePath).mockResolvedValueOnce(null);
    vi.mocked(buildRom).mockClear();
    await build();
    expect(saveScript).not.toHaveBeenCalled();
    expect(buildRom).not.toHaveBeenCalled();
  });

  it("build_with_a_known_path_builds_there_without_asking_and_resolves_to_the_rom", async () => {
    await act(async () => void (await session.onBuild("/kept/out.nds")));
    expect(savePath).not.toHaveBeenCalled();
    expect(buildRom).toHaveBeenCalledExactlyOnceWith("/kept/out.nds");
    // Without one it still asks, and resolves to what was built (or null when nothing was).
    let rom: string | null = "unset";
    await act(async () => void (rom = await session.onBuild()));
    expect(savePath).toHaveBeenCalledOnce();
    expect(rom).toBe("/out.nds");
    vi.mocked(savePath).mockResolvedValueOnce(null);
    await act(async () => void (rom = await session.onBuild()));
    expect(rom).toBeNull();
  });

  it("run_starts_the_given_rom_and_falls_back_to_the_last_one_built", async () => {
    localStorage.setItem("g4.emulatorPath", "/emu");
    vi.mocked(runRom).mockResolvedValue(1);
    await act(async () => session.onRun("/given.nds"));
    expect(runRom).toHaveBeenLastCalledWith("/emu", "/given.nds");
    await build();
    await act(async () => session.onRun());
    expect(runRom).toHaveBeenLastCalledWith("/emu", "/out.nds");
    localStorage.removeItem("g4.emulatorPath");
  });

  it("keeps_the_chosen_emulator_path_and_sets_it_after_pick_emulator", async () => {
    expect(session.emulatorPath).toBeNull();
    vi.mocked(openPath).mockResolvedValueOnce("/Applications/melonDS.app");
    await act(async () => session.onPickEmulator());
    expect(session.emulatorPath).toBe("/Applications/melonDS.app");
    expect(localStorage.getItem("g4.emulatorPath")).toBe("/Applications/melonDS.app");
    // Picking nothing keeps the choice.
    vi.mocked(openPath).mockResolvedValueOnce(null);
    await act(async () => session.onPickEmulator());
    expect(session.emulatorPath).toBe("/Applications/melonDS.app");
    localStorage.removeItem("g4.emulatorPath");
  });

  it("reads_the_stored_emulator_path_when_it_starts", async () => {
    localStorage.setItem("g4.emulatorPath", "/emu");
    act(() => root.unmount());
    root = createRoot(container);
    await act(async () => root.render(<Host />));
    expect(session.emulatorPath).toBe("/emu");
    localStorage.removeItem("g4.emulatorPath");
  });

  it("run_forgets_the_stored_emulator_when_it_fails_to_start_and_says_so", async () => {
    localStorage.setItem("g4.emulatorPath", "/falsch");
    vi.mocked(runRom).mockRejectedValueOnce({ code: "error", message: "Emulator nicht startbar", asset: null });
    await act(async () => session.onRun("/given.nds"));
    expect(localStorage.getItem("g4.emulatorPath")).toBeNull();
    expect(session.emulatorPath).toBeNull();
    expect(session.message).toMatchObject({ kind: "error" });
    expect(session.message?.text).toBe("Emulator nicht startbar. Beim nächsten Start fragt das Studio erneut nach dem Emulator.");
  });

  it("build_puts_compile_problems_of_the_saves_into_the_store_and_still_builds", async () => {
    await openDirty(3);
    vi.mocked(saveScript).mockResolvedValue({ problems: [problem("expanded/scripts/0003.rotom", 2, "unbekannter Befehl", "warning")], touched_archives: [] });
    await build();
    expect(problems.list().map((p) => p.message)).toEqual(["unbekannter Befehl"]);
    expect(buildRom).toHaveBeenCalledTimes(1);
  });

  it("a_successful_build_clears_the_stale_compile_errors_and_keeps_warnings", async () => {
    problems.replaceForFile("expanded/scripts/0007.rotom", [problem("expanded/scripts/0007.rotom", 3, "alter Fehler")]);
    problems.replaceForFile("expanded/scripts/0003.rotom", [problem("expanded/scripts/0003.rotom", 2, "Warnung", "warning")]);
    await build();
    expect(problems.list().map((p) => p.message)).toEqual(["Warnung"]);
    expect(session.message?.text).toContain("ROM gebaut");
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
    expect(session.message).toMatchObject({ kind: "error", text: "2 Fehler in Scripts, Build abgebrochen" });
    expect(session.message?.actions?.map((a) => a.label)).toEqual(["Probleme zeigen"]);
    session.message?.actions?.[0]?.run();
    expect(showProblems).toHaveBeenCalledOnce();
    expect(session.busy).toBe(false);
    expect(session.lastRom).toBeNull();
    expect(session.lastBuiltAt).toBeNull();
  });

  it("one_compile_error_of_the_build_is_counted_as_1_Fehler", async () => {
    vi.mocked(buildRom).mockRejectedValue({ code: "compile", message: "1 Fehler in Scripts oder Texten", asset: null, problems: [problem("expanded/scripts/0003.rotom", 2, "e1")] });
    await build();
    expect(session.message).toMatchObject({ kind: "error", text: "1 Fehler in Scripts, Build abgebrochen" });
  });

  it("a_successful_build_stamps_the_time_and_opening_a_project_forgets_it", async () => {
    const before = Date.now();
    await build();
    expect(session.lastRom).toBe("/out.nds");
    expect(session.lastBuiltAt).toBeGreaterThanOrEqual(before);
    expect(session.lastBuiltAt).toBeLessThanOrEqual(Date.now());
    vi.mocked(openProject).mockResolvedValue({ root: "/q", game_code: "CPUE", family: "Pt", language: "English", header_count: 0 });
    await act(async () => void (await session.openProjectAt("/q", () => {})));
    expect(session.lastRom).toBeNull();
    expect(session.lastBuiltAt).toBeNull();
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
      vi.mocked(openPath).mockResolvedValue("/p");
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 0 });
      await act(async () => session.onOpen(() => {}));
    });

    it("save_writes_the_store_and_then_the_open_scripts_and_counts_both", async () => {
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [{ kind: "TextArchive", index: 5 }, { kind: "Map", index: 1 }], problems: [problem("expanded/textArchives/0005.json", 1, "zu breit", "warning", "text")] });
      await openDirty(3);
      await act(async () => session.onSave());
      expect(saveScript).toHaveBeenCalledWith(3, "script F3 #1:\n// edited\n");
      expect(vi.mocked(save).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(saveScript).mock.invocationCallOrder[0]!);
      expect(session.message).toMatchObject({ kind: "info", text: "3 Datei(en) gespeichert" });
      expect(problems.list().map((p) => p.message)).toEqual(["zu breit"]);
      expect(events).toContainEqual({ topic: "lspFilesChanged", changes: [{ path: "expanded/textArchives/0005.json", type: 2 }] });
      // The next save of archive 5 without warnings clears them.
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [{ kind: "TextArchive", index: 5 }], problems: [] });
      await act(async () => session.onSave());
      expect(problems.list()).toEqual([]);
    });

    it("a_header_save_that_compiles_replaces_the_problems_of_the_compiled_files", async () => {
      // The script's text literal did not resolve before the header was saved.
      problems.applyReport([problem("expanded/scripts/1124.rotom", 7, "Textliteral nicht auflösbar: …")], { files: ["expanded/scripts/1124.rotom"] });
      problems.applyReport([problem("expanded/scripts/0042.rotom", 2, "Unknown command 'X'")], { files: ["expanded/scripts/0042.rotom"] });
      vi.mocked(save).mockResolvedValue({
        written: [{ kind: "HeaderTable", index: 0 }],
        problems: [problem("expanded/scripts/0042.rotom", 2, "Unknown command 'X'"), problem("expanded/textArchives/0023.json", 1, "msg_00101: zu breit", "warning", "text")],
        compiled: ["expanded/scripts/1124.rotom"],
        touched_archives: [23],
      });
      await act(async () => session.onSave());
      expect(problems.list().map((p) => p.message)).toEqual(["Unknown command 'X'", "msg_00101: zu breit"]);
      expect(events).toContainEqual({ topic: "textData", archive: 23 });
      expect(events).toContainEqual({ topic: "lspFilesChanged", changes: [{ path: "expanded/textArchives/0023.json", type: 2 }] });
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
