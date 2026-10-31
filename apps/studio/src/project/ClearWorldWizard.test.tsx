// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyEmptyRegion, nameTables, planEmptyRegion } from "../api";
import type { AdoptProgressDto, AdoptReportDto, NameTablesDto, RegionPlan, RegionReport } from "../api";
import { ClearWorldWizard } from "./ClearWorldWizard";
import type { ClearWorldWizardProps } from "./ClearWorldWizard";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), planEmptyRegion: vi.fn(), applyEmptyRegion: vi.fn(), nameTables: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

/** Platin DE as the kernel plans it: Zweiblattdorf with six rooms. */
const PLAN: RegionPlan = {
  start_header: 411,
  town_name: "Zweiblattdorf",
  town_cells: [[3, 27]],
  town_map: 0,
  town_altitude: 0,
  town_event_file: 390,
  town_script_file: 1052,
  kept: [411, 412, 413, 414, 415, 416, 417],
  emptied: Array.from({ length: 536 }, (_, i) => i),
  border_cells: 467,
  empty_level_script: 897,
  dropped_warps: [],
  removed_objects: 8,
  removed_triggers: 2,
  starter_species: 387,
  starter_level: 5,
  already_empty: false,
};
const REPORT: RegionReport = { plan: PLAN, border_map: 666, level_script: 1234, start_script: [1052, 10], change: 7, changed: [{ kind: "Matrix", index: 0 }] };
const ADOPTED = { adopted: true, file_count: 10 };
const NOT_ADOPTED = { adopted: false, file_count: 10 };
const species = Array.from({ length: 494 }, (_, i) => `Art${i}`);
species[387] = "Chelast";
const NAMES = { species, moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: {} } as unknown as NameTablesDto;

describe("ClearWorldWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = {
    onAdopt: vi.fn<ClearWorldWizardProps["onAdopt"]>(),
    onSave: vi.fn<ClearWorldWizardProps["onSave"]>(),
    onApplied: vi.fn<ClearWorldWizardProps["onApplied"]>(),
    onClose: vi.fn<ClearWorldWizardProps["onClose"]>(),
    onShow: vi.fn<ClearWorldWizardProps["onShow"]>(),
    onPlay: vi.fn<NonNullable<ClearWorldWizardProps["onPlay"]>>(),
    onStep: vi.fn<NonNullable<ClearWorldWizardProps["onStep"]>>(),
  };
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(planEmptyRegion).mockReset().mockResolvedValue(PLAN);
    vi.mocked(applyEmptyRegion).mockReset().mockResolvedValue(REPORT);
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    for (const fn of Object.values(handlers)) fn.mockReset();
    handlers.onSave.mockResolvedValue(undefined);
    handlers.onApplied.mockResolvedValue(undefined);
    handlers.onShow.mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const wizard = (over: Partial<ClearWorldWizardProps> = {}) => <ClearWorldWizard scripts={ADOPTED} unsaved={0} {...handlers} {...over} />;
  async function show(over: Partial<ClearWorldWizardProps> = {}) {
    await act(async () => root.render(wizard(over)));
    await flush();
  }
  const text = () => container.textContent ?? "";
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");

  it("walks_plan_and_summary_then_applies", async () => {
    await show();
    expect(planEmptyRegion).toHaveBeenCalledOnce();
    expect(current()).toBe("1. Was bleibt");
    expect(text()).toContain("Zweiblattdorf bleibt mit 6 Räumen.");
    expect(text()).toContain("536 Orte werden geleert: ihre Figuren, Türen, Auslöser und Level-Scripts verschwinden. Scripts und Texte bleiben liegen.");
    expect(text()).toContain("467 Felder der Weltkarte werden zu einer gesperrten Randkarte.");
    expect(text()).toContain("Ein Start-Script gibt dir Chelast auf Level 5, den Pokédex und die Laufschuhe.");
    expect(button("Weiter")?.disabled).toBe(false);
    await click(button("Weiter"));

    expect(current()).toBe("2. Zusammenfassung");
    expect(text()).toContain("Leert die Welt bis auf Zweiblattdorf. Die Welt holst du mit ⌘Z zurück; Randkarte, Level-Script und Start-Script bleiben als Dateien.");
    expect(text()).toContain("Nicht rückgängig");
    expect(applyEmptyRegion).not.toHaveBeenCalled();
    await click(button("Welt leeren"));
    await flush();

    expect(applyEmptyRegion).toHaveBeenCalledOnce();
    // The world is read again at once, like after ⌘Z.
    expect(handlers.onApplied).toHaveBeenCalledExactlyOnceWith(REPORT);
    expect(text()).toContain("Welt geleert");
    expect(text()).toContain("Randkarte 0666, Level-Script 1234, Start-Script 1052 #10");
    expect(text()).toContain("Mit „Hier spielen“: Intro, zu Hause aufwachen, draußen wartet dein erstes Pokémon.");
    await click(button("Schließen"));
    // The world was read again already.
    expect(handlers.onClose).toHaveBeenCalledExactlyOnceWith(false);
  });

  it("offers_to_adopt_scripts_first", async () => {
    // Without the name tables the species goes by its number.
    vi.mocked(nameTables).mockRejectedValue({ code: "error", message: "no names", asset: null });
    let finish: (report: AdoptReportDto) => void = () => {};
    let progress: (p: AdoptProgressDto) => void = () => {};
    handlers.onAdopt.mockImplementation((onProgress) => {
      progress = onProgress;
      return new Promise((resolve) => (finish = resolve));
    });
    await show({ scripts: NOT_ADOPTED });
    expect(text()).toContain("Ein Start-Script gibt dir Art 387 auf Level 5, den Pokédex und die Laufschuhe.");
    expect(text()).toContain("Dafür müssen die Scripts übernommen sein.");
    expect(button("Weiter")?.disabled).toBe(true);

    await click(button("Scripts übernehmen"));
    expect(handlers.onAdopt).toHaveBeenCalledOnce();
    await act(async () => progress({ done: 3, total: 10 }));
    expect(container.querySelector('[role="progressbar"]')?.getAttribute("aria-valuenow")).toBe("3");
    expect(button("Scripts übernehmen")).toBeUndefined();
    await act(async () => finish({ decompiled: 9, skipped_empty: 1, failed: [] }));
    // The app's script status says adopted now.
    await act(async () => root.render(wizard({ scripts: ADOPTED })));
    expect(text()).not.toContain("Dafür müssen die Scripts übernommen sein.");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("asks_to_save_first", async () => {
    await show({ unsaved: 2 });
    expect(text()).toContain("Speichere zuerst, das Rezept schreibt Scripts direkt auf die Platte.");
    expect(button("Weiter")?.disabled).toBe(true);
    await click(button("Speichern"));
    expect(handlers.onSave).toHaveBeenCalledOnce();
    await act(async () => root.render(wizard({ unsaved: 0 })));
    expect(text()).not.toContain("Speichere zuerst");
    expect(button("Weiter")?.disabled).toBe(false);

    // The kernel's refusal (something changed in between) is said in German, and the wizard stays.
    await click(button("Weiter"));
    vi.mocked(applyEmptyRegion).mockRejectedValue({ code: "error", message: "save first", asset: null });
    await click(button("Welt leeren"));
    await flush();
    expect(text()).toContain("Speichere zuerst, das Rezept schreibt Scripts direkt auf die Platte.");
    expect(handlers.onApplied).not.toHaveBeenCalled();
    expect(current()).toBe("2. Zusammenfassung");
  });

  it("a_failed_run_blocks_another", async () => {
    // Not a known refusal: the kernel may have written the border map and scripts before it failed.
    vi.mocked(applyEmptyRegion).mockRejectedValue({ code: "error", message: "compile failed", asset: null });
    await show();
    await click(button("Weiter"));
    await click(button("Welt leeren"));
    await flush();
    expect(applyEmptyRegion).toHaveBeenCalledOnce();
    expect(text()).toContain("compile failed");
    expect(current()).toBe("2. Zusammenfassung");
    // The plan is stale: no second run from this wizard, also not after stepping back and forth.
    expect(button("Welt leeren")?.disabled).toBe(true);
    await click(button("Zurück"));
    expect(button("Weiter")?.disabled).toBe(true);
    expect(text()).toContain("compile failed");
    await click(button("Abbrechen"));
    // The app reads the project again; reopening the wizard plans again.
    expect(handlers.onClose).toHaveBeenCalledExactlyOnceWith(true);
    expect(applyEmptyRegion).toHaveBeenCalledOnce();
  });

  it("already_empty_blocks", async () => {
    vi.mocked(planEmptyRegion).mockResolvedValue({ ...PLAN, emptied: [], border_cells: 0, already_empty: true });
    await show({ scripts: NOT_ADOPTED });
    expect(text()).toContain("Die Welt ist schon leer.");
    // Nothing else is to be done first.
    expect(text()).not.toContain("Dafür müssen die Scripts übernommen sein.");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("shows_a_plan_that_is_refused", async () => {
    vi.mocked(planEmptyRegion).mockRejectedValue({ code: "error", message: "Diamant/Perl: ohne Testprojekt nicht freigegeben", asset: null });
    await show();
    expect(text()).toContain("Diamant/Perl: ohne Testprojekt nicht freigegeben");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("shows_the_result_with_show_and_play", async () => {
    await show();
    await click(button("Weiter"));
    await click(button("Welt leeren"));
    await flush();
    await click(button("Zeigen"));
    expect(handlers.onShow).toHaveBeenCalledExactlyOnceWith(REPORT);
    expect(handlers.onPlay).not.toHaveBeenCalled();
    await click(button("Hier spielen"));
    await flush();
    expect(handlers.onShow).toHaveBeenCalledTimes(2);
    expect(handlers.onPlay).toHaveBeenCalledOnce();
  });

  it("the_result_offers_gebiet_anlegen_and_opens_it", async () => {
    await show();
    await click(button("Weiter"));
    await click(button("Welt leeren"));
    await flush();
    const area = container.querySelector<HTMLButtonElement>('button[data-recipe="create_area"]')!;
    expect(area.textContent).toBe("Gebiet anlegen");
    expect(area.disabled).toBe(false);
    await click(area);
    expect(handlers.onStep).toHaveBeenCalledExactlyOnceWith("create_area", REPORT);
  });
});
