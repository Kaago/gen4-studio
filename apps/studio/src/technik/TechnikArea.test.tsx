// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderPatchDto } from "../api";
import { applyHeaderPatch, headerPatchStatus, repairHeaderPatch } from "../api";
import { createToolStatus } from "../app/toolStatus";
import type { ToolStatusStore } from "../app/toolStatus";
import { ProblemsStore } from "../workbench/problems/problems";
import { TechnikArea } from "./TechnikArea";
import type { TechnikAreaProps, TechnikEntry } from "./TechnikArea";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  headerPatchStatus: vi.fn(),
  applyHeaderPatch: vi.fn(),
  repairHeaderPatch: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const patch = (over: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders", ...over });

describe("TechnikArea", () => {
  let container: HTMLElement;
  let root: Root;
  let problems: ProblemsStore;
  let tools: ToolStatusStore;
  const handlers = { onBuild: vi.fn(), onRun: vi.fn(), onPickEmulator: vi.fn(), onOpenProject: vi.fn(), onToggleList: vi.fn(), goTo: vi.fn(), onHeaderPatch: vi.fn(async () => {}) };

  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    problems = new ProblemsStore();
    tools = createToolStatus();
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue(patch());
    vi.mocked(applyHeaderPatch).mockReset().mockResolvedValue(patch({ applied: true }));
    vi.mocked(repairHeaderPatch).mockReset().mockResolvedValue(patch({ applied: true }));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** The area with its entry kept like App keeps it. */
  function Host({ start = "bauen", ...over }: Partial<TechnikAreaProps> & { start?: TechnikEntry }) {
    const [entry, setEntry] = useState<TechnikEntry>(start);
    return (
      <TechnikArea
        project="/p"
        listOpen
        problems={problems}
        depth="bauen"
        busy={false}
        canRun
        lastBuiltAt={null}
        lastRom={null}
        emulatorPath={null}
        tools={tools}
        {...handlers}
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

  it("lists_four_entries_with_symbol_and_status_and_the_list_switches_the_surface", async () => {
    await render({ lastBuiltAt: Date.now() - 4 * 60_000, lastRom: "/out.nds" });
    expect(container.querySelector("[role=listbox]")?.getAttribute("aria-label")).toBe("Technik");
    expect(rows().map((r) => document.getElementById(r.getAttribute("aria-labelledby")!)?.textContent)).toEqual(["Prüfen und Bauen", "Patches", "Emulator", "Werkzeuge"]);
    for (const r of rows()) expect(r.querySelector("svg")).not.toBeNull();
    expect(subtitle("Prüfen und Bauen")).toBe("gebaut vor 4 Minuten");
    expect(subtitle("Patches")).toBe("Dynamische Header verfügbar");
    expect(subtitle("Emulator")).toBe("Kein Emulator gewählt");
    expect(subtitle("Werkzeuge")).toBe("0 von 2 bereit");

    expect(row("Prüfen und Bauen")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("bauen");
    await click(row("Patches"));
    expect(row("Patches")?.getAttribute("aria-selected")).toBe("true");
    expect(surface()?.getAttribute("data-technik")).toBe("patches");
    expect(surface()?.querySelector("h1")?.textContent).toBe("Patches");
    await click(row("Emulator"));
    expect(surface()?.getAttribute("data-technik")).toBe("emulator");
    await click(row("Werkzeuge"));
    expect(surface()?.getAttribute("data-technik")).toBe("werkzeuge");
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
    expect(button("Bauen")?.disabled).toBe(true);
    expect(button("Starten")?.disabled).toBe(true);

    // The browser mode starts no emulator (the bridge refuses, and the refusal would forget the chosen one).
    await render({ canRun: false, lastRom: "/roms/out.nds" });
    expect(button("Starten")?.disabled).toBe(true);
    expect(surface()?.textContent).toContain("Im Browser-Modus wird nur gebaut");
  });

  it("pruefen_und_bauen_says_when_there_are_no_problems", async () => {
    await render();
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
    vi.mocked(applyHeaderPatch).mockRejectedValue({ code: "error", message: "Erst die Header speichern", asset: null });
    await render({ start: "patches" });
    await settle();
    await click(button("Patch anwenden"));
    await click(button("Patch anwenden", dialog()!));
    await settle();
    expect(surface()?.textContent).toContain("Erst die Header speichern");
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
});
