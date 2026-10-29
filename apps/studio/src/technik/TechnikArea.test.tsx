// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderPatchDto, PreviewDto } from "../api";
import { applyHeaderPatch, baseStatus, createPatch, dirty, headerPatchStatus, projectPreviewGet, projectPreviewSet, repairHeaderPatch } from "../api";
import { createToolStatus } from "../app/toolStatus";
import type { ToolStatusStore } from "../app/toolStatus";
import { fakeChecks, finding } from "../checks/checksTesting";
import type { ChecksApi } from "../checks/useChecks";
import { openPath } from "../platform/dialogs";
import { previewWritten } from "../project/previewMessages";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { ProblemsStore, findingId } from "../workbench/problems/problems";
import { MISSING, READY, resetSpy, withReset } from "../original/resetTesting";
import type { OriginalStatus } from "../api";
import { found, missing, patchDto } from "./baseTesting";
import { changesWith, entry, report } from "./changesTesting";
import { conflict, outcome, syncWith, textWith } from "./textProjectTesting";
import { useBase } from "./useBase";
import { TechnikArea } from "./TechnikArea";
import type { TechnikAreaProps, TechnikEntry } from "./TechnikArea";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  headerPatchStatus: vi.fn(),
  applyHeaderPatch: vi.fn(),
  repairHeaderPatch: vi.fn(),
  dirty: vi.fn(),
  projectPreviewGet: vi.fn(),
  projectPreviewSet: vi.fn(),
  baseStatus: vi.fn(),
  createPatch: vi.fn(),
}));
vi.mock("../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../platform/dialogs")>()), openPath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** The start town of the recipe: Zweiblattdorf's header in Platin. */
const TOWN = { header: 411, map: 3, areaDataId: 7 };
const preview = (source: PreviewDto["source"], png: string | null = source === null ? null : `png-${source}`, standardMap: PreviewDto["standardMap"] = TOWN): PreviewDto => ({ source, png, standardMap });

const patch = (over: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders", ...over });

describe("TechnikArea", () => {
  let container: HTMLElement;
  let root: Root;
  let problems: ProblemsStore;
  let checks: ChecksApi;
  let tools: ToolStatusStore;
  const handlers = { onBuild: vi.fn(), onBuildFresh: vi.fn(), onRun: vi.fn(), onPickEmulator: vi.fn(), onOpenProject: vi.fn(), onToggleList: vi.fn(), goTo: vi.fn(), onHeaderPatch: vi.fn(async () => {}), onClearWorld: vi.fn() };

  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    problems = new ProblemsStore();
    checks = fakeChecks();
    tools = createToolStatus();
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue(patch());
    vi.mocked(applyHeaderPatch).mockReset().mockResolvedValue({ ...patch({ applied: true }), snapshot: null });
    vi.mocked(repairHeaderPatch).mockReset().mockResolvedValue(patch({ applied: true }));
    vi.mocked(dirty).mockReset().mockResolvedValue([]);
    vi.mocked(projectPreviewGet).mockReset().mockResolvedValue(preview(null));
    vi.mocked(projectPreviewSet).mockReset();
    vi.mocked(openPath).mockReset();
    resetToasts();
    vi.mocked(baseStatus).mockReset().mockResolvedValue({ state: "unassigned", suggestion: "/Sinnoh-Remix.nds" });
    vi.mocked(createPatch).mockReset().mockResolvedValue(patchDto());
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** The area with its entry kept like App keeps it. */
  function Host({ start = "bauen", ...over }: Partial<TechnikAreaProps> & { start?: TechnikEntry }) {
    const [entry, setEntry] = useState<TechnikEntry>(start);
    // The app holds the base ROM above the areas (as App does).
    const project = over.project === undefined ? "/p" : over.project;
    const base = useBase({ project, active: true });
    return (
      <TechnikArea
        project="/p"
        listOpen
        problems={problems}
        checks={checks}
        depth="bauen"
        busy={false}
        canRun
        lastBuiltAt={null}
        lastRom={null}
        lastSteps={null}
        emulatorPath={null}
        tools={tools}
        changes={null}
        dirty={[]}
        textProject={null}
        {...handlers}
        base={project === null ? null : base}
        {...over}
        entry={entry}
        onEntry={setEntry}
      />
    );
  }
  const render = (props: Partial<TechnikAreaProps> & { start?: TechnikEntry } = {}) => act(async () => root.render(<Host {...props} />));
  const settle = () => act(async () => {});
  const rows = () => [...container.querySelectorAll<HTMLElement>("[role=listbox] [role=option]")];
  const row = (label: string) => rows().find((r) => document.getElementById(r.getAttribute("aria-labelledby") ?? "")?.textContent === label);
  const subtitle = (label: string) => document.getElementById(row(label)?.getAttribute("aria-describedby") ?? "")?.textContent;
  const surface = () => container.querySelector<HTMLElement>("[data-technik]");
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => b.textContent === name);
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("lists_the_entries_with_symbol_and_status_veroeffentlichen_after_pruefen_and_the_list_switches_the_surface", async () => {
    await render({ lastBuiltAt: Date.now() - 4 * 60_000, lastRom: "/out.nds" });
    await settle();
    expect(container.querySelector("[role=listbox]")?.getAttribute("aria-label")).toBe("Technik");
    expect(rows().map((r) => document.getElementById(r.getAttribute("aria-labelledby")!)?.textContent)).toEqual(["Projekt", "Textprojekt", "Prüfen und Bauen", "Veröffentlichen", "Änderungen", "Patches", "Emulator", "Werkzeuge"]);
    for (const r of rows()) expect(r.querySelector("svg")).not.toBeNull();
    expect(subtitle("Prüfen und Bauen")).toBe("gebaut vor 4 Minuten");
    expect(subtitle("Veröffentlichen")).toBe("Keine Basis-ROM zugeordnet");
    expect(subtitle("Patches")).toBe("Dynamische Header verfügbar");
    expect(subtitle("Emulator")).toBe("Kein Emulator gewählt");
    expect(subtitle("Werkzeuge")).toBe("0 von 2 bereit");

    expect(row("Prüfen und Bauen")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("bauen");
    await click(row("Veröffentlichen"));
    expect(row("Veröffentlichen")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("veroeffentlichen");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Veröffentlichen");
    expect(surface()?.querySelector("[data-base-state]")?.getAttribute("data-base-state")).toBe("unassigned");
    // The status was read once for the list and the surface together.
    expect(baseStatus).toHaveBeenCalledOnce();
    await click(row("Patches"));
    expect(row("Patches")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("patches");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Patches");
    await click(row("Emulator"));
    expect(surface()?.getAttribute("data-technik")).toBe("emulator");
    await click(row("Werkzeuge"));
    expect(surface()?.getAttribute("data-technik")).toBe("werkzeuge");
    await click(row("Projekt"));
    expect(surface()?.getAttribute("data-technik")).toBe("projekt");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Projekt");
  });

  it("lists_aenderungen_after_veroeffentlichen", async () => {
    const reset = resetSpy();
    const changes = changesWith({ report: report({ entries: [entry()] }) });
    await act(async () => root.render(withReset(reset, <Host changes={changes} />)));
    const labels = rows().map((r) => document.getElementById(r.getAttribute("aria-labelledby")!)?.textContent);
    expect(labels).toHaveLength(8);
    expect(labels.indexOf("Änderungen")).toBe(labels.indexOf("Veröffentlichen") + 1);
    expect(row("Änderungen")?.querySelector("svg")).not.toBeNull();
    await click(row("Änderungen"));
    expect(row("Änderungen")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("aenderungen");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Änderungen");
    expect(surface()?.querySelector("[data-change='Karte 0003']")).not.toBeNull();
    // The rows reset through the app's dialog.
    await click([...surface()!.querySelectorAll("button")].find((b) => b.textContent === "Auf Original zurücksetzen…"));
    expect(reset.open).toHaveBeenCalledExactlyOnceWith({ kind: "map", index: 3 }, { finalFocus: expect.any(Function) });
  });

  it("lists_eight_entries_with_textprojekt_after_projekt", async () => {
    const onSetUp = vi.fn();
    await render({ textProject: textWith({ status: { state: "dspre", adoptable: true, reason: null }, onSetUp }) });
    const labels = rows().map((r) => document.getElementById(r.getAttribute("aria-labelledby")!)?.textContent);
    expect(labels).toHaveLength(8);
    expect(labels.indexOf("Textprojekt")).toBe(labels.indexOf("Projekt") + 1);
    expect(row("Textprojekt")?.querySelector("svg")).not.toBeNull();
    await click(row("Textprojekt"));
    expect(row("Textprojekt")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("textprojekt");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Textprojekt");
    await click(button("Als Textprojekt einrichten…"));
    expect(onSetUp).toHaveBeenCalledOnce();
  });

  it("textprojekt_subtitle_counts_conflicts_then_broken_files", async () => {
    const show = (textProject: TechnikAreaProps["textProject"]) => render({ textProject });
    await show(null);
    expect(subtitle("Textprojekt")).toBe("Kein Projekt geöffnet");
    await show(textWith({ status: null }));
    expect(subtitle("Textprojekt")).toBe("Wird gelesen …");
    await show(textWith({ status: { state: "dspre", adoptable: true, reason: null } }));
    expect(subtitle("Textprojekt")).toBe("DSPRE-Layout");
    await show(textWith({ sync: syncWith({ running: true }) }));
    expect(subtitle("Textprojekt")).toBe("Gleicht ab …");
    await show(textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict()] }) }) }));
    expect(subtitle("Textprojekt")).toBe("1\u00a0Konflikt");
    await show(textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict(), conflict({ key: "moves/0002" }), conflict({ key: "moves/0003" })] }) }) }));
    expect(subtitle("Textprojekt")).toBe("3\u00a0Konflikte");
    // UI test 4.3b U5: broken files are counted after the conflicts, never „Abgeglichen“.
    const broken = (key: string) => ({ key, label: key, problems: [] });
    await show(textWith({ sync: syncWith({ last: outcome({ broken: [broken("moves/0039")] }) }) }));
    expect(subtitle("Textprojekt")).toBe("1\u00a0fehlerhafte Datei");
    await show(textWith({ sync: syncWith({ last: outcome({ broken: [broken("moves/0039"), broken("moves/0040")] }) }) }));
    expect(subtitle("Textprojekt")).toBe("2\u00a0fehlerhafte Dateien");
    await show(textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict()], broken: [broken("moves/0039")] }) }) }));
    expect(subtitle("Textprojekt")).toBe("1\u00a0Konflikt");
    // UI test 4.3b U4: an agent sets the DSPRE folder up.
    await show(textWith({ status: { state: "dspre", adoptable: true, reason: null }, agentSetUp: true }));
    expect(subtitle("Textprojekt")).toBe("Wird eingerichtet …");
    // A sync under way says so even over the conflicts it found before.
    await show(textWith({ sync: syncWith({ running: true, last: outcome({ conflicts: [conflict()] }) }) }));
    expect(subtitle("Textprojekt")).toBe("Gleicht ab …");
    await show(textWith());
    expect(subtitle("Textprojekt")).toBe("Abgeglichen");
    await show(textWith({ status: { state: "older", schema: 0 } }));
    expect(subtitle("Textprojekt")).toBe("Schema veraltet");
    await show(textWith({ status: { state: "newer", schema: 9 } }));
    expect(subtitle("Textprojekt")).toBe("Nicht lesbar");
    await show(textWith({ status: { state: "broken", reason: "g4-project.json:1:1: kaputt" } }));
    expect(subtitle("Textprojekt")).toBe("Nicht lesbar");
  });

  it("aenderungen_subtitle_counts_the_entries", async () => {
    const show = (changes: TechnikAreaProps["changes"], status: OriginalStatus | null = READY, project?: string | null) =>
      act(async () => root.render(withReset(resetSpy(status), <Host changes={changes} {...(project === undefined ? {} : { project })} />)));
    await show(null, null, null);
    expect(subtitle("Änderungen")).toBe("Kein Projekt geöffnet");
    await show(changesWith({ report: null }), MISSING);
    expect(subtitle("Änderungen")).toBe("Basis-ROM fehlt");
    await show(changesWith({ report: null, busy: true }));
    expect(subtitle("Änderungen")).toBe("Vergleiche …");
    // A later compare runs over the report before: still „Vergleiche …“.
    await show(changesWith({ report: report({ entries: [entry()] }), busy: true }));
    expect(subtitle("Änderungen")).toBe("Vergleiche …");
    await show(changesWith({ report: report() }));
    expect(subtitle("Änderungen")).toBe("Keine Abweichung");
    await show(changesWith({ report: report({ entries: [entry()] }) }));
    expect(subtitle("Änderungen")).toBe("1 Eintrag weicht ab");
    await show(changesWith({ report: report({ entries: [entry(), entry({ label: "Karte 0004" }), entry({ label: "Karte 0005" })] }) }));
    expect(subtitle("Änderungen")).toBe("3 Einträge weichen ab");
  });

  it("veroeffentlichen_subtitle_follows_the_base_state", async () => {
    // Before the answer: reading.
    vi.mocked(baseStatus).mockReturnValue(new Promise(() => {}));
    await render();
    expect(subtitle("Veröffentlichen")).toBe("Wird gelesen …");
    const cases: [Awaited<ReturnType<typeof baseStatus>>, string][] = [
      [{ state: "unassigned", suggestion: null }, "Keine Basis-ROM zugeordnet"],
      [missing(), "Basis-ROM fehlt"],
      [{ state: "unreadable", reason: "kaputt" }, "Basismarke nicht lesbar"],
      [found(), "Bereit für einen Patch"],
    ];
    for (const [status, text] of cases) {
      act(() => root.render(<></>));
      vi.mocked(baseStatus).mockResolvedValue(status);
      await render();
      await settle();
      expect(subtitle("Veröffentlichen")).toBe(text);
    }
    // After a patch.
    await click(row("Veröffentlichen"));
    await click(button("Patch erstellen…"));
    await click(button("Patch erstellen", dialog()!));
    await settle();
    expect(subtitle("Veröffentlichen")).toBe("Patch erstellt");
    // Without a project.
    act(() => root.render(<></>));
    await render({ project: null });
    expect(subtitle("Veröffentlichen")).toBe("Kein Projekt geöffnet");
  });

  it("says_not_built_yet_and_shows_the_chosen_emulator_and_the_ready_tools_in_the_list", async () => {
    tools.set({ lsp: { state: "bereit" }, apicula: "bereit" });
    await render({ emulatorPath: "/Applications/melonDS.app" });
    expect(subtitle("Prüfen und Bauen")).toBe("Noch nicht gebaut");
    expect(subtitle("Emulator")).toBe("/Applications/melonDS.app");
    expect(subtitle("Werkzeuge")).toBe("2 von 2 bereit");
    // The store tells the area when a tool changes.
    await act(async () => tools.set({ apicula: "fehlt" }));
    expect(subtitle("Werkzeuge")).toBe("1 von 2 bereit");
  });

  it("pruefen_und_bauen_shows_the_problems_the_buttons_and_the_last_build", async () => {
    problems.applyReport([{ severity: "error", file: "expanded/scripts/0042.rotom", line: 3, col: 1, message: "unbekannter Befehl", source: "compile" }]);
    await render();
    expect(surface()?.querySelector("table")?.textContent).toContain("unbekannter Befehl");
    expect(surface()?.textContent).toContain("Noch nicht gebaut");
    expect(button("Starten")?.disabled).toBe(true);
    await click(button("Bauen"));
    expect(handlers.onBuild).toHaveBeenCalledOnce();

    await render({ lastBuiltAt: Date.now() - 2 * 60_000, lastRom: "/roms/out.nds" });
    expect(surface()?.textContent).toContain("vor 2 Minuten");
    expect(surface()?.textContent).toContain("/roms/out.nds");
    await click(button("Starten"));
    expect(handlers.onRun).toHaveBeenCalledOnce();
    // A link in the table goes where the problem is.
    await click(surface()?.querySelector("table button"));
    expect(handlers.goTo).toHaveBeenCalledWith({ kind: "script", file: 42, line: 3 });

    // While a build runs, neither button starts another.
    await render({ busy: true, lastRom: "/roms/out.nds" });
    // Busy blocks with aria-disabled and a guard (the button that started the build keeps keyboard focus), not with `disabled`.
    handlers.onBuild.mockClear();
    handlers.onRun.mockClear();
    for (const name of ["Bauen", "Starten"]) {
      expect(button(name)?.getAttribute("aria-disabled")).toBe("true");
      await click(button(name));
    }
    expect(handlers.onBuild).not.toHaveBeenCalled();
    expect(handlers.onRun).not.toHaveBeenCalled();
    button("Bauen")!.focus();
    expect(document.activeElement).toBe(button("Bauen"));
    // „Starten“ without a ROM stays `disabled`, busy or not.
    await render({ busy: true });
    expect(button("Starten")?.disabled).toBe(true);

    // The browser mode starts no emulator (the bridge refuses, and the refusal would forget the chosen one).
    await render({ canRun: false, lastRom: "/roms/out.nds" });
    expect(button("Starten")?.disabled).toBe(true);
    expect(surface()?.textContent).toContain("Im Browser-Modus wird nur gebaut");
  });

  it("rebuild_without_cache_calls_on_build_fresh", async () => {
    await render();
    const fresh = button("Neu bauen (ohne Cache)");
    expect(fresh?.getAttribute("title")).toBe("Löscht den Bauzustand und baut alles neu");
    await click(fresh);
    expect(handlers.onBuildFresh).toHaveBeenCalledOnce();
    expect(handlers.onBuild).not.toHaveBeenCalled();
    // While a build runs, it starts none.
    await render({ busy: true });
    expect(button("Neu bauen (ohne Cache)")?.getAttribute("aria-disabled")).toBe("true");
    await click(button("Neu bauen (ohne Cache)"));
    expect(handlers.onBuildFresh).toHaveBeenCalledOnce();
  });

  it("shows_the_steps_of_the_last_build", async () => {
    await render();
    const heading = [...(surface()?.querySelectorAll("h2") ?? [])].find((h) => h.textContent === "Schritte");
    expect(heading).toBeDefined();
    expect(surface()?.textContent).toContain("Noch kein Build in dieser Sitzung.");
    expect(surface()?.querySelector("[data-step]")).toBeNull();

    const step = { status: "skipped" as const, ms: 0, done: 0, total: 0, cached: 0, reason: null };
    await render({
      lastBuiltAt: Date.now(),
      lastRom: "/roms/out.nds",
      lastSteps: [
        { ...step, id: "pack:maps", status: "incremental", ms: 21, done: 1, total: 540 },
        { ...step, id: "pack:textArchives" },
        { ...step, id: "rom", status: "full", ms: 50 },
      ],
    });
    expect(surface()?.textContent).not.toContain("Noch kein Build in dieser Sitzung.");
    expect([...(surface()?.querySelectorAll<HTMLElement>("[data-step]") ?? [])].map((r) => `${r.dataset.step}=${r.dataset.status}`)).toEqual(["pack:maps=incremental", "rom=full"]);
    expect(surface()?.textContent).toContain("Übersprungen (1)");
    // The steps come after the last build's facts and before the problems.
    const headings = [...(surface()?.querySelectorAll("h2") ?? [])].map((h) => h.textContent);
    expect(headings.slice(0, 2)).toEqual(["Schritte", "Probleme"]);
  });

  it("names_the_patch_build_under_schritte_when_the_last_steps_are_its_and_keeps_the_path_of_the_last_rom", async () => {
    const step = { status: "full" as const, ms: 50, done: 0, total: 0, cached: 0, reason: null };
    const built = { lastBuiltAt: Date.now(), lastRom: "/roms/out.nds", lastSteps: [{ ...step, id: "rom" }] };
    const line = () => surface()?.querySelector("[data-steps-source]");
    await render(built);
    expect(line()).toBeNull();
    await render({ ...built, lastPatchRom: "/p/build/4-2-abn-patch-IPKD.nds" });
    expect(line()?.textContent).toBe("Build von „Patch erstellen“ nach build/4-2-abn-patch-IPKD.nds");
    // Directly under the heading, before the table; the fact „Pfad“ still names the ROM of Bauen.
    const heading = [...(surface()?.querySelectorAll("h2") ?? [])].find((h) => h.textContent === "Schritte")!;
    expect(heading.nextElementSibling).toBe(line());
    expect(line()?.nextElementSibling?.querySelector("table")).not.toBeNull();
    expect(surface()?.querySelector("dd[title]")?.textContent).toBe("/roms/out.nds");
    // A ROM outside the project shows whole; without steps there is nothing to name.
    await render({ ...built, lastPatchRom: "/elsewhere/x.nds" });
    expect(line()?.textContent).toBe("Build von „Patch erstellen“ nach /elsewhere/x.nds");
    await render({ lastPatchRom: "/p/build/x.nds" });
    expect(line()).toBeNull();
  });

  it("pruefen_und_bauen_shows_the_findings_in_the_same_table_as_the_problems_panel", async () => {
    const never = finding("warp.never-fires", "info", { key: "warp:5", args: ["5", "8", "4", "0x00", "0x00"] });
    problems.replaceChecks([finding("warp.dest-warp", "error", { args: ["2", "414", "5", "411", "3"] }), never]);
    await render();
    const groups = [...surface()!.querySelectorAll<HTMLElement>("[role=group][data-check]")].map((g) => g.dataset.check);
    expect(groups).toEqual(["warp.dest-warp", "warp.never-fires"]);
    // The same words as the panel at the same depth (Bauen here).
    expect(surface()?.textContent).toContain("Die Tür führt zu Warp 5 von Ort 0414");
    await click(button("Ausblenden", surface()!.querySelector(`[data-finding="${findingId(never)}"]`)!));
    expect(checks.ignore).toHaveBeenCalledWith(never);
    await click(button("Erneut prüfen"));
    expect(checks.recheck).toHaveBeenCalledOnce();
  });

  it("ausgangslage_uebernehmen_asks_with_the_number_first", async () => {
    problems.replaceChecks([
      finding("warp.dest-warp", "error"),
      finding("warp.never-fires", "info", { key: "warp:5" }),
      finding("event.trigger-empty", "warning", { key: "trigger:0" }),
      finding("event.same-spot", "warning", { key: "object:3" }),
    ]);
    await render();
    await click(button("Ausgangslage übernehmen…"));
    const asked = dialog()!;
    expect(asked.querySelector("h2")?.textContent).toBe("Ausgangslage übernehmen?");
    expect(asked.textContent).toContain("3 Warnungen und Hinweise werden ausgeblendet");
    await click(button("Abbrechen", asked));
    expect(dialog()).toBeNull();
    expect(checks.acceptBaseline).not.toHaveBeenCalled();
    await click(button("Ausgangslage übernehmen…"));
    await click(button("Übernehmen", dialog()!));
    expect(checks.acceptBaseline).toHaveBeenCalledOnce();
    expect(dialog()).toBeNull();
  });

  it("switches_a_check_off_and_on_and_never_one_that_reports_errors", async () => {
    await render();
    await click(button("Prüfungen ein- und ausschalten"));
    const list = surface()!.querySelector<HTMLElement>("[role=list][aria-label='Prüfungen ein- und ausschalten']")!;
    const toggle = (code: string) => list.querySelector<HTMLElement>(`[data-check="${code}"] [role=switch]`)!;
    expect(list.querySelectorAll("[data-check]")).toHaveLength(45);
    expect(toggle("encounter.empty-slot").getAttribute("aria-checked")).toBe("true");
    await click(toggle("encounter.empty-slot"));
    expect(checks.setEnabled).toHaveBeenCalledWith("encounter.empty-slot", false);
    await act(async () => checks.feed.update({ disabled: ["encounter.empty-slot"] }));
    expect(toggle("encounter.empty-slot").getAttribute("aria-checked")).toBe("false");
    await click(toggle("encounter.empty-slot"));
    expect(checks.setEnabled).toHaveBeenLastCalledWith("encounter.empty-slot", true);
    // Errors cannot be switched off (`warp.dest-warp` is one).
    const dest = toggle("warp.dest-warp");
    expect(dest.getAttribute("aria-disabled") === "true" || dest.hasAttribute("data-disabled")).toBe(true);
    expect(list.querySelector('[data-check="warp.dest-warp"]')?.textContent).toContain("Fehler lassen sich nicht abschalten");
    // A header of another matrix is an error too (ruling R16).
    const elsewhere = toggle("matrix.header-elsewhere");
    expect(elsewhere.getAttribute("aria-disabled") === "true" || elsewhere.hasAttribute("data-disabled")).toBe(true);
  });

  it("pruefen_und_bauen_says_when_there_are_no_problems", async () => {
    await render();
    // Before the first report: checking, or not checked; never „Keine Probleme“.
    await act(async () => checks.feed.update({ running: true }));
    expect(surface()?.textContent).toContain("Prüft das Projekt …");
    expect(surface()?.textContent).not.toContain("Keine Probleme");
    await act(async () => checks.feed.update({ running: false }));
    expect(surface()?.textContent).toContain("Nicht geprüft.");
    expect(surface()?.textContent).not.toContain("Keine Probleme");
    await act(async () => checks.feed.update({ reported: true }));
    expect(surface()?.querySelector("table")).toBeNull();
    expect(surface()?.textContent).toContain("Keine Probleme");
  });

  it("shows_a_plaque_for_each_patch_state", async () => {
    const plaque = () => surface()?.querySelector("[data-patch]");
    const cases: [HeaderPatchDto, string, string][] = [
      [patch({ applied: true }), "applied", "Angewendet"],
      [patch(), "available", "Verfügbar"],
      [patch({ applied: true, repairable: true }), "broken", "Fehlerhaft"],
      [patch({ supported: false, narc: null }), "unsupported", "Nur Platin und HGSS"],
    ];
    for (const [status, state, text] of cases) {
      vi.mocked(headerPatchStatus).mockResolvedValue(status);
      // A fresh area per state: the status is read when it shows.
      act(() => root.render(<></>));
      await render({ start: "patches" });
      await settle();
      expect(plaque()?.getAttribute("data-patch")).toBe(state);
      expect(plaque()?.textContent).toBe(text);
    }
    // Neither button for a game without the patch.
    expect(button("Patch anwenden")).toBeUndefined();
    expect(button("Patch reparieren")).toBeUndefined();
  });

  it("list_subtitle_follows_the_patch_state", async () => {
    vi.mocked(headerPatchStatus).mockResolvedValue(patch({ applied: true }));
    await render();
    await settle();
    expect(subtitle("Patches")).toBe("Dynamische Header angewendet");
    act(() => root.render(<></>));
    vi.mocked(headerPatchStatus).mockResolvedValue(patch({ applied: true, repairable: true }));
    await render();
    await settle();
    expect(subtitle("Patches")).toBe("Dynamische Header fehlerhaft");
    act(() => root.render(<></>));
    vi.mocked(headerPatchStatus).mockResolvedValue(patch({ supported: false }));
    await render();
    await settle();
    expect(subtitle("Patches")).toBe("Nur Platin und HGSS");
  });

  it("patch_anwenden_applies_with_unsaved_headers_and_says_they_stay_unsaved", async () => {
    vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable", index: 0 }]);
    await render({ start: "patches" });
    await settle();
    await click(button("Patch anwenden"));
    expect(dialog()?.textContent).toContain(
      "Der Patch ändert Bytes im Spielcode (arm9.bin), die Sicherung arm9.bin.backup wird angelegt. Ungespeicherte Änderungen an Gebieten und Räumen bleiben ungespeichert.",
    );
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(applyHeaderPatch).toHaveBeenCalledOnce();
    expect(handlers.onHeaderPatch).toHaveBeenCalledOnce();
    expect(surface()?.textContent).not.toContain("ungespeicherte Änderungen");
    expect(surface()?.querySelector("[data-patch]")?.textContent).toBe("Angewendet");
  });

  it("patch_anwenden_asks_first_and_applies_only_on_yes", async () => {
    await render({ start: "patches" });
    await settle();
    await click(button("Patch anwenden"));
    expect(applyHeaderPatch).not.toHaveBeenCalled();
    const asked = dialog()!;
    expect(document.getElementById(asked.getAttribute("aria-labelledby")!)?.textContent).toBe("Patch „Dynamische Header“ anwenden?");
    expect(asked.textContent).toContain("Nicht rückgängig");
    await click(button("Abbrechen", asked));
    expect(dialog()).toBeNull();
    expect(applyHeaderPatch).not.toHaveBeenCalled();

    await click(button("Patch anwenden"));
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(applyHeaderPatch).toHaveBeenCalledOnce();
    expect(handlers.onHeaderPatch).toHaveBeenCalledOnce();
    expect(dialog()).toBeNull();
    expect(surface()?.querySelector("[data-patch]")?.textContent).toBe("Angewendet");
    expect(button("Patch anwenden")).toBeUndefined();
  });

  it("patch_anwenden_names_where_the_old_header_folder_went", async () => {
    vi.mocked(applyHeaderPatch).mockResolvedValue({ ...patch({ applied: true }), snapshot: "/p/.g4/backup/dynamicHeaders-20261026-120000" });
    await render({ start: "patches" });
    await settle();
    expect(surface()?.textContent).not.toContain("Sicherung des alten Ordners");
    await click(button("Patch anwenden"));
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(surface()?.textContent).toContain("Sicherung des alten Ordners: /p/.g4/backup/dynamicHeaders-20261026-120000");
  });

  it("patch_anwenden_without_an_old_folder_names_no_snapshot", async () => {
    await render({ start: "patches" });
    await settle();
    await click(button("Patch anwenden"));
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(surface()?.querySelector("[data-patch]")?.textContent).toBe("Angewendet");
    expect(surface()?.textContent).not.toContain("Sicherung des alten Ordners");
  });

  it("patch_reparieren_repairs_a_broken_patch_and_a_refusal_shows_its_reason", async () => {
    vi.mocked(headerPatchStatus).mockResolvedValue(patch({ applied: true, repairable: true }));
    await render({ start: "patches" });
    await settle();
    await click(button("Patch reparieren"));
    await settle();
    expect(repairHeaderPatch).toHaveBeenCalledOnce();
    expect(surface()?.querySelector("[data-patch]")?.textContent).toBe("Angewendet");

    act(() => root.render(<></>));
    vi.mocked(headerPatchStatus).mockResolvedValue(patch());
    vi.mocked(applyHeaderPatch).mockRejectedValue({ code: "error", message: "Die Header-Tabelle hat sich auf der Platte geändert", asset: null });
    await render({ start: "patches" });
    await settle();
    await click(button("Patch anwenden"));
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(surface()?.textContent).toContain("Die Header-Tabelle hat sich auf der Platte geändert");
    expect(handlers.onHeaderPatch).not.toHaveBeenCalled();
  });

  it("without_a_project_pruefen_and_patches_show_an_empty_state_and_read_nothing", async () => {
    await render({ project: null });
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    await click(button("Projekt öffnen"));
    expect(handlers.onOpenProject).toHaveBeenCalledOnce();
    await click(row("Patches"));
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    expect(headerPatchStatus).not.toHaveBeenCalled();
    expect(subtitle("Patches")).toBe("Kein Projekt geöffnet");
    // Projekt too: no picture is read.
    await click(row("Projekt"));
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    expect(button("Vorschaubild wählen…")).toBeUndefined();
    expect(projectPreviewGet).not.toHaveBeenCalled();
    expect(subtitle("Projekt")).toBe("Kein Projekt geöffnet");
    await click(row("Veröffentlichen"));
    expect(surface()?.textContent).toContain("Öffne ein Projekt, dann erstellt Technik hier seinen Patch.");
    expect(baseStatus).not.toHaveBeenCalled();
    // The emulator does not need a project.
    await click(row("Emulator"));
    await click(button("Emulator wählen"));
    expect(handlers.onPickEmulator).toHaveBeenCalledOnce();
  });

  it("werkzeuge_shows_apicula_and_rotom_lsp_with_their_state_and_restarts_a_stopped_server", async () => {
    await render({ start: "werkzeuge" });
    const lsp = () => surface()!.querySelector<HTMLElement>("[data-lsp]");
    expect(surface()?.textContent).toContain("apicula");
    expect(surface()?.textContent).toContain("rotom-lsp");
    // The panel "Script" was never shown: no server was started.
    expect(lsp()?.getAttribute("data-lsp")).toBe("none");
    expect(lsp()?.textContent).toContain("Nicht gestartet");
    expect(surface()?.querySelector("[data-tool=apicula]")?.textContent).toContain("Unbekannt");

    const restart = vi.fn();
    await act(async () => tools.set({ lsp: { state: "fehler", note: "rotom-lsp antwortet nicht", restart }, apicula: "fehlt" }));
    expect(lsp()?.getAttribute("data-lsp")).toBe("fehler");
    expect(lsp()?.textContent).toContain("rotom-lsp antwortet nicht");
    expect(surface()?.querySelector("[data-tool=apicula]")?.textContent).toContain("Fehlt");
    await click(button("Neu starten", lsp()!));
    expect(restart).toHaveBeenCalledOnce();

    await act(async () => tools.set({ lsp: { state: "bereit", restart } }));
    expect(lsp()?.getAttribute("data-lsp")).toBe("bereit");
    expect(button("Neu starten", lsp()!)).toBeUndefined();
  });

  it("werkzeuge_says_why_rotom_lsp_is_off_only_when_the_script_panel_said_so", async () => {
    await render({ start: "werkzeuge" });
    const lsp = () => surface()!.querySelector<HTMLElement>("[data-lsp]");
    // Off without a reason (the panel went away with adopted scripts): a neutral note, no promise about the adoption.
    await act(async () => tools.set({ lsp: { state: "aus" } }));
    expect(lsp()?.getAttribute("data-lsp")).toBe("aus");
    expect(lsp()?.textContent).toContain("Nicht gestartet.");
    expect(lsp()?.textContent).not.toContain("übernommen");
    // Off because the scripts are not adopted: the panel's reason.
    await act(async () => tools.set({ lsp: { state: "aus", note: "Läuft, sobald die Scripts des Projekts übernommen sind." } }));
    expect(lsp()?.textContent).toContain("Läuft, sobald die Scripts des Projekts übernommen sind.");
  });
  it("clear_world_needs_a_project", async () => {
    await render({ start: "werkzeuge", project: null });
    const tool = surface()!.querySelector<HTMLElement>("[data-tool=welt-leeren]");
    expect(tool?.textContent).toContain("Welt leeren");
    expect(tool?.textContent).toContain("Macht aus der Welt eine leere Region mit Startstadt. Die Welt holst du mit ⌘Z zurück.");
    expect(tool?.textContent).toContain("Erst ein Projekt öffnen");
    expect(button("Welt leeren…", tool!)?.disabled).toBe(true);
    await click(button("Welt leeren…", tool!));
    expect(handlers.onClearWorld).not.toHaveBeenCalled();
  });

  it("clear_world_opens_the_wizard", async () => {
    await render({ start: "werkzeuge" });
    const tool = surface()!.querySelector<HTMLElement>("[data-tool=welt-leeren]");
    expect(tool?.textContent).not.toContain("Erst ein Projekt öffnen");
    expect(button("Welt leeren…", tool!)?.disabled).toBe(false);
    await click(button("Welt leeren…", tool!));
    expect(handlers.onClearWorld).toHaveBeenCalledOnce();
  });

  describe("Projekt", () => {
    const section = () => surface()!.querySelector<HTMLElement>("[data-section=vorschaubild]");
    const picture = () => section()?.querySelector("img");
    const hint = () => section()?.querySelector("[data-hint]")?.textContent;
    const toasts = () => listToasts().map((t) => [t.kind, t.text]);

    it("Projekt shows the preview and \"Vorschaubild wählen…\"", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("standard"));
      await render({ start: "projekt", project: "/roms/pt-a" });
      // The open project's picture (no path), in the section „Vorschaubild“ on a DS screen of 256 × 192.
      expect(projectPreviewGet).toHaveBeenCalledWith();
      expect(surface()?.getAttribute("data-technik")).toBe("projekt");
      expect(section()?.querySelector("h2")?.textContent).toBe("Vorschaubild");
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-standard");
      expect(picture()?.getAttribute("width")).toBe("256");
      expect(picture()?.getAttribute("height")).toBe("192");
      expect(button("Vorschaubild wählen…", section()!)?.getAttribute("data-kind")).toBe("secondary");
      expect(subtitle("Projekt")).toBe("Kartenbild der Startstadt");
    });

    it("without a picture shows the project's name on the empty screen", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview(null, null, null));
      await render({ start: "projekt", project: "/roms/pt-a" });
      expect(picture()).toBeNull();
      expect(section()?.textContent).toContain("pt-a");
      expect(subtitle("Projekt")).toBe("Kein Vorschaubild");
      // A failing read is no picture too, without a message.
      vi.mocked(projectPreviewGet).mockRejectedValue(new Error("bridge down"));
      await render({ start: "projekt", project: "/roms/pt-b" });
      expect(picture()).toBeNull();
      expect(section()?.textContent).toContain("pt-b");
      expect(toasts()).toEqual([]);
    });

    it("\"Entfernen\" only with an own picture", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("standard"));
      await render({ start: "projekt" });
      expect(button("Entfernen", section()!)).toBeUndefined();

      vi.mocked(projectPreviewGet).mockResolvedValue(preview("eigen"));
      await render({ start: "projekt", project: "/q" });
      expect(subtitle("Projekt")).toBe("Eigenes Vorschaubild");
      const remove = button("Entfernen", section()!);
      expect(remove?.getAttribute("data-kind")).toBe("secondary");
      // Removing writes the open project's own slot empty; the standard picture shows again and „Entfernen“ goes.
      vi.mocked(projectPreviewSet).mockResolvedValue(preview("standard"));
      await click(remove);
      await settle();
      expect(openPath).not.toHaveBeenCalled();
      expect(projectPreviewSet).toHaveBeenCalledWith({});
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-standard");
      expect(button("Entfernen", section()!)).toBeUndefined();
      expect(toasts()).toEqual([["info", "Vorschaubild entfernt"]]);
    });

    it("keeps_keyboard_focus_meaningful_while_a_write_runs_and_after_it", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("eigen"));
      await render({ start: "projekt", project: "/q" });
      // Busy blocks the actions with aria-disabled, not `disabled`: the focused button keeps focus while the write runs.
      let answer: (dto: ReturnType<typeof preview>) => void = () => {};
      vi.mocked(projectPreviewSet).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
      const remove = button("Entfernen", section()!) as HTMLButtonElement;
      remove.focus();
      await click(remove);
      expect(document.activeElement).toBe(remove);
      expect(remove.disabled).toBe(false);
      expect(remove.getAttribute("aria-disabled")).toBe("true");
      expect(button("Vorschaubild wählen…", section()!)?.getAttribute("aria-disabled")).toBe("true");
      // A second activation while busy does nothing.
      await click(remove);
      await click(button("Vorschaubild wählen…", section()!));
      expect(projectPreviewSet).toHaveBeenCalledOnce();
      expect(openPath).not.toHaveBeenCalled();
      // „Entfernen“ is gone with the write: focus moves to „Vorschaubild wählen…“, not to the body.
      await act(async () => answer(preview("standard")));
      expect(button("Entfernen", section()!)).toBeUndefined();
      expect(document.activeElement).toBe(button("Vorschaubild wählen…", section()!));
      expect(button("Vorschaubild wählen…", section()!)?.hasAttribute("aria-disabled")).toBe(false);
    });

    it("focus_rests_on_choose_after_choosing_whether_set_cancelled_or_failed", async () => {
      await render({ start: "projekt" });
      const choose = () => button("Vorschaubild wählen…", section()!) as HTMLButtonElement;
      const pick = async () => {
        choose().focus();
        await click(choose());
        await settle();
        expect(document.activeElement).toBe(choose());
      };
      vi.mocked(openPath).mockResolvedValueOnce("/bilder/stadt.jpg");
      vi.mocked(projectPreviewSet).mockResolvedValueOnce(preview("eigen"));
      await pick();
      vi.mocked(openPath).mockResolvedValueOnce(null);
      await pick();
      vi.mocked(openPath).mockResolvedValueOnce("/notiz.txt");
      vi.mocked(projectPreviewSet).mockRejectedValueOnce({ code: "error", message: "preview.format: neither PNG nor JPEG", asset: null });
      await pick();
      // The native dialog took focus to the body meanwhile: it comes back to the button.
      vi.mocked(openPath).mockImplementationOnce(async () => {
        (document.activeElement as HTMLElement).blur();
        return null;
      });
      await pick();
    });

    it("the hint names the start town", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("standard"));
      const startTown = vi.fn((header: number) => (header === 411 ? "Zweiblattdorf" : null));
      await render({ start: "projekt", startTown });
      expect(startTown).toHaveBeenCalledWith(411);
      expect(hint()).toBe("Ohne eigenes Bild zeigt Start das Kartenbild von Zweiblattdorf.");
      // No name for the header (the world does not know it), no start town (no recipe), no lookup at all: the general sentence.
      await render({ start: "projekt", startTown: () => null, project: "/q" });
      expect(hint()).toBe("Ohne eigenes Bild zeigt Start das Kartenbild der Startstadt.");
      vi.mocked(projectPreviewGet).mockResolvedValue(preview(null, null, null));
      await render({ start: "projekt", startTown, project: "/r" });
      expect(hint()).toBe("Ohne eigenes Bild zeigt Start das Kartenbild der Startstadt.");
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("standard"));
      await render({ start: "projekt", startTown: undefined, project: "/s" });
      expect(hint()).toBe("Ohne eigenes Bild zeigt Start das Kartenbild der Startstadt.");
    });

    it("shows_a_picture_set_from_the_scene_at_once", async () => {
      vi.mocked(projectPreviewGet).mockResolvedValue(preview("standard"));
      await render({ start: "projekt", startTown: () => "Zweiblattdorf" });
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-standard");
      // „Als Vorschaubild“ in the scene wrote the own picture while Technik shows: it is there without a new read.
      await act(async () => previewWritten.emit(preview("eigen", "png-szene", null)));
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-szene");
      expect(button("Entfernen", section()!)).toBeDefined();
      expect(projectPreviewGet).toHaveBeenCalledOnce();
      expect(subtitle("Projekt")).toBe("Eigenes Vorschaubild");
      // The start town stays known for the hint, though the write's answer had none.
      expect(hint()).toBe("Ohne eigenes Bild zeigt Start das Kartenbild von Zweiblattdorf.");
    });

    it("choosing calls set with file and shows the toast", async () => {
      await render({ start: "projekt" });
      vi.mocked(openPath).mockResolvedValueOnce("/bilder/stadt.jpg");
      vi.mocked(projectPreviewSet).mockResolvedValueOnce(preview("eigen"));
      await click(button("Vorschaubild wählen…", section()!));
      await settle();
      expect(openPath).toHaveBeenCalledWith({ directory: false, title: "Vorschaubild wählen", filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }] });
      expect(projectPreviewSet).toHaveBeenCalledWith({ file: "/bilder/stadt.jpg" });
      expect(toasts()).toEqual([["info", "Vorschaubild gesetzt"]]);
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-eigen");
      expect(button("Entfernen", section()!)).toBeDefined();
      expect(subtitle("Projekt")).toBe("Eigenes Vorschaubild");

      // Cancelled: nothing written, nothing said. Refused: the sentence of its code, the picture stays.
      resetToasts();
      vi.mocked(projectPreviewSet).mockClear();
      vi.mocked(openPath).mockResolvedValueOnce(null);
      await click(button("Vorschaubild wählen…", section()!));
      await settle();
      expect(projectPreviewSet).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
      vi.mocked(openPath).mockResolvedValueOnce("/notiz.txt");
      vi.mocked(projectPreviewSet).mockRejectedValueOnce({ code: "error", message: "preview.format: neither PNG nor JPEG", asset: null });
      await click(button("Vorschaubild wählen…", section()!));
      await settle();
      expect(toasts()).toEqual([["error", "Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei."]]);
      expect(picture()?.getAttribute("src")).toBe("data:image/png;base64,png-eigen");
    });
  });
});
