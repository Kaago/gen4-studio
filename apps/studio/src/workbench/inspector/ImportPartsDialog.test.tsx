// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { importMapParts, inspectMapImport, worldFreeformAdd, worldFreeformInspect, worldGet, worldImportTmj, worldInspectTmj } from "../../api";
import type { CheckProblemDto, FreeformInspect, ImportPreview, PartReport, TmjImportReport, WorldBakeReport, WorldGetDto } from "../../api";
import { fakeChecks, finding } from "../../checks/checksTesting";
import type { ChecksApi } from "../../checks/useChecks";
import { typeInto } from "../../ui/primitives/testUtils";
import { openFiles } from "../../platform/dialogs";
import { ImportPartsDialog } from "./ImportPartsDialog";

vi.mock("../../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../platform/dialogs")>()), openFiles: vi.fn() }));
vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  inspectMapImport: vi.fn(),
  importMapParts: vi.fn(),
  worldGet: vi.fn(),
  worldFreeformInspect: vi.fn(),
  worldFreeformAdd: vi.fn(),
  worldInspectTmj: vi.fn(),
  worldImportTmj: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const reports: PartReport[] = [
  {
    part: "model",
    file: null,
    size: 41236,
    state: { kind: "ok" },
    notes: [
      { code: "model.bdhc-unchanged", args: [], message: "terrain (BDHC) stays unchanged; check walking heights" },
      { code: "model.textures-missing", args: ["3", "7", "a, b, c"], message: "3 texture or palette names missing in map texture pack 7: a, b, c" },
    ],
  },
  { part: "permissions", file: null, size: 2048, state: { kind: "ok" }, notes: [] },
  { part: "buildings", file: null, size: 50, state: { kind: "error", code: "buildings.size", args: ["50", "48"], message: "buildings are 50 bytes, not a multiple of 48" }, notes: [] },
];

const worldDto = (world: boolean): WorldGetDto => ({
  map: 5,
  world: world ? { version: 2, tileset: 6, layers: [], permissions: { derive: true } } : null,
  faces: 0,
  textures: 0,
  estimate: 0,
  share: 0,
  bake: null,
});

const worldBake = (over: Partial<WorldBakeReport> = {}): WorldBakeReport => ({
  map: 5,
  ok: true,
  style: "outdoor",
  faces: 12,
  model_bytes: 21740,
  model_share: 0.35,
  model_kept: false,
  textures: [],
  vram_bytes: 0,
  bdhc_bytes: 350,
  bdhc_payload: 334,
  plates: 4,
  blocked: 0,
  verbatim_model: false,
  verbatim_terrain: false,
  messages: [],
  freeform_faces: 6,
  strips: 2,
  single_quads: 0,
  ...over,
});

describe("ImportPartsDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onImported = vi.fn();
  const goTo = vi.fn();
  let checks: ChecksApi;
  beforeEach(() => {
    checks = fakeChecks();
    goTo.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(inspectMapImport).mockReset().mockResolvedValue(reports);
    vi.mocked(importMapParts).mockReset().mockResolvedValue([{ kind: "Map", index: 5 }]);
    vi.mocked(worldGet).mockReset().mockResolvedValue(worldDto(false));
    vi.mocked(worldFreeformInspect).mockReset();
    vi.mocked(worldFreeformAdd).mockReset();
    vi.mocked(worldInspectTmj).mockReset();
    vi.mocked(worldImportTmj).mockReset();
    onClose.mockReset();
    onImported.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement | undefined;
  const box = (name: string) => dialog().querySelector<HTMLElement>(`[role=checkbox][aria-label='${name}']`)!;
  async function show() {
    await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} checks={checks} goTo={goTo} />));
    const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
    await act(async () => void field.focus());
    typeInto(field, "6");
    await act(async () => void field.blur());
    await flush();
  }

  it("lists_the_parts_of_the_source_with_their_state_and_german_hints", async () => {
    await show();
    expect(inspectMapImport).toHaveBeenCalledWith(5, { map: 6 });
    const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
    expect([...list.querySelectorAll("li")].map((r) => (r as HTMLElement).dataset.state)).toEqual(["ok", "ok", "error"]);
    expect(list.textContent).toContain("BDHC bleibt unverändert, Laufhöhen prüfen");
    expect(list.textContent).toContain("3 Texturnamen fehlen im Gebiets-Paket 7");
    expect(list.textContent).toContain("41.236 Bytes");
    // A part that cannot be imported says why in German, with the way out.
    expect(list.textContent).toContain("Die Gebäude sind 50 Byte groß, kein Vielfaches von 48. Wähle eine vollständige .bld-Datei.");
    expect(list.textContent).not.toContain("not a multiple");
  });

  it("does_not_preselect_an_empty_part_and_says_it_is_left_out_but_keeps_it_choosable", async () => {
    vi.mocked(inspectMapImport).mockResolvedValue([
      { part: "permissions", file: null, size: 2048, state: { kind: "ok" }, notes: [] },
      { part: "buildings", file: null, size: 0, state: { kind: "ok" }, notes: [] },
      { part: "bgs", file: null, size: 0, state: { kind: "ok" }, notes: [] },
    ]);
    await show();
    const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
    expect([...list.querySelectorAll("li")].map((r) => (r as HTMLElement).dataset.state)).toEqual(["ok", "info", "info"]);
    expect(list.textContent).toContain("Leer, wird nicht übernommen");
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("false");
    expect(box("Hintergrundklang").getAttribute("aria-checked")).toBe("false");
    expect(button("1 Teil übernehmen")).toBeDefined();
    await act(async () => box("Gebäude").click());
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("true");
    expect(button("2 Teile übernehmen")).toBeDefined();
  });

  it("keeps_a_part_with_an_error_from_being_chosen_and_names_the_count_on_the_button", async () => {
    await show();
    expect(box("Gebäude").getAttribute("aria-disabled") === "true" || box("Gebäude").hasAttribute("data-disabled")).toBe(true);
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("false");
    expect(button("2 Teile übernehmen")).toBeDefined();
    await act(async () => box("Kollision").click());
    expect(button("1 Teil übernehmen")).toBeDefined();
  });

  it("imports_the_chosen_parts_and_says_how_many_were_taken_over", async () => {
    await show();
    await act(async () => box("Kollision").click());
    await act(async () => button("1 Teil übernehmen")!.click());
    await flush();
    expect(importMapParts).toHaveBeenCalledWith(5, { map: 6 }, ["model"], false);
    expect(onImported).toHaveBeenCalledWith(5);
    expect(dialog().textContent).toContain("1 Teil übernommen");
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalled();
  });

  it("offers_to_take_over_buildings_only_for_a_whole_map_of_the_other_family", async () => {
    await show();
    expect(dialog().textContent).not.toContain("Gebäude übernehmen");
    vi.mocked(inspectMapImport).mockResolvedValue([
      { part: "whole", file: "/x/m.bin", size: 9000, state: { kind: "ok" }, notes: [{ code: "whole.foreign", args: ["Hgss"], message: "HeartGold/SoulSilver map file; its buildings are left out unless taken over" }] },
    ]);
    const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
    await act(async () => void field.focus());
    typeInto(field, "7");
    await act(async () => void field.blur());
    await flush();
    const toggle = dialog().querySelector<HTMLElement>("[role=switch]")!;
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    await act(async () => toggle.click());
    await act(async () => button("1 Teil übernehmen")!.click());
    await flush();
    expect(importMapParts).toHaveBeenCalledWith(5, { map: 7 }, ["whole"], true);
  });

  it("checks_files_typed_one_per_line", async () => {
    await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} />));
    await act(async () => button("Datei")!.click());
    const area = dialog().querySelector<HTMLTextAreaElement>("textarea")!;
    await act(async () => void area.focus());
    typeInto(area, "/a/m.nsbmd\n\n/a/m.per\n");
    await act(async () => void area.blur());
    await flush();
    expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/a/m.nsbmd", "/a/m.per"] });
  });

  describe("converted models and images", () => {
    const preview: ImportPreview = {
      pack: 0,
      area: 0,
      style: "outdoor",
      vram_bytes: 23544,
      vram_limit: 110640,
      textures: [{ name: "g4_imp_cube", file: "/p/g4_imp_cube.png", width: 16, height: 16, colors: 16, new: true, mean_error: 7.6 }],
      model: { format: "obj", stats: { polygons: 6, quads: 6, triangles: 0, dropped: 0 }, strips: 2, single_quads: 0, budget: { used: 21740, max: 61440, state: "calm" } },
      files: ["/p/a.obj"],
    };
    const cube: PartReport = {
      part: "model",
      file: "/p/a.obj",
      size: 21740,
      state: { kind: "ok" },
      notes: [{ code: "model.converted", args: ["OBJ", "6", "6", "0", "2", "0"], message: "OBJ model converted" }],
      preview,
    };
    const units = () => dialog().querySelector<HTMLInputElement>("input[aria-label='Einheit']");
    const colours = () => dialog().querySelector<HTMLElement>("[aria-label='Farben je Textur']");
    async function showFiles(paths: string, tinker = false) {
      await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} tinker={tinker} />));
      await act(async () => button("Datei")!.click());
      const area = dialog().querySelector<HTMLTextAreaElement>("textarea")!;
      await act(async () => void area.focus());
      typeInto(area, paths);
      await act(async () => void area.blur());
      await flush();
    }

    it("a_model_file_shows_the_options_and_the_preview", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([cube]);
      await showFiles("/p/a.obj");
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/a.obj"] }, { unit: 16, colors: 16 });
      const convert = dialog().querySelector<HTMLElement>("[data-import-convert]")!;
      expect(convert.textContent).toContain("Umwandeln");
      expect(convert.textContent).toContain("Farben je Textur");
      expect(colours()).not.toBeNull();
      expect(units()!.value).toBe("16");
      expect(convert.textContent).toContain("Welteinheiten je Modelleinheit; 16 = eine Kachel");
      const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
      expect(list.textContent).toContain("OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen");
      expect(list.textContent).toContain("g4_imp_cube · 16 × 16 · 16 Farben · neu");
      expect(list.textContent).toContain("VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B");
      const budget = list.querySelector<HTMLElement>("[data-budget-state]")!;
      expect(budget.dataset.budgetState).toBe("calm");
      expect(budget.textContent).toContain("Modell");
      expect(budget.querySelector("[role=meter]")!.getAttribute("aria-valuetext")).toBe("21.740 von 61.440 B");
      expect(budget.querySelector("[data-tech]")).toBeNull();
      expect(box("Modell").getAttribute("aria-checked")).toBe("true");
      expect(button("1 Teil übernehmen")).toBeDefined();
    });

    it("shows_the_limit_in_hex_when_tinkering", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([cube]);
      await showFiles("/p/a.obj", true);
      expect(dialog().querySelector("[data-budget-state] [data-tech]")!.textContent).toBe("0xF000");
    });

    it("changing_colours_inspects_again_with_options", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([cube]);
      await showFiles("/p/a.obj");
      expect(inspectMapImport).toHaveBeenCalledTimes(1);
      const four = [...colours()!.querySelectorAll("button")].find((b) => b.textContent === "4")!;
      await act(async () => four.click());
      await flush();
      expect(inspectMapImport).toHaveBeenCalledTimes(2);
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/a.obj"] }, { unit: 16, colors: 4 });
      // A new unit inspects again; one out of range says so and asks nothing.
      await act(async () => void units()!.focus());
      typeInto(units()!, "0,5");
      await act(async () => void units()!.blur());
      await flush();
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/a.obj"] }, { unit: 0.5, colors: 4 });
      await act(async () => void units()!.focus());
      typeInto(units()!, "300");
      await act(async () => void units()!.blur());
      await flush();
      expect(dialog().textContent).toContain("Zwischen 0,0625 und 256.");
      expect(inspectMapImport).toHaveBeenCalledTimes(3);
    });

    it("a_png_only_shows_colours_not_unit", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([
        { part: "textures", file: null, size: 512, state: { kind: "ok" }, notes: [], preview: { ...preview, model: null, files: ["/p/gras.png"] } },
      ]);
      await showFiles("/p/gras.png");
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/gras.png"] }, { colors: 16 });
      expect(dialog().textContent).toContain("Umwandeln");
      expect(colours()).not.toBeNull();
      expect(units()).toBeNull();
      expect(dialog().querySelector("[data-budget-state]")).toBeNull();
      expect(box("Texturen").getAttribute("aria-checked")).toBe("true");
    });

    it("shows_no_options_for_parts_that_are_not_converted", async () => {
      await showFiles("/p/a.nsbmd");
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/a.nsbmd"] });
      expect(dialog().querySelector("[data-import-convert]")).toBeNull();
    });

    it("an_oversize_model_is_not_selectable_and_says_why", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([
        {
          ...cube,
          size: 139264,
          state: { kind: "error", code: "model.too-big", args: ["139264", "61440"], message: "model is 139264 bytes" },
          preview: { ...preview, model: { ...preview.model!, budget: { used: 139264, max: 61440, state: "full" } } },
        },
      ]);
      await showFiles("/p/a.obj");
      const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
      expect(list.textContent).toContain("Das Modell ist 139.264 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.");
      expect(box("Modell").getAttribute("aria-disabled") === "true" || box("Modell").hasAttribute("data-disabled")).toBe(true);
      expect(box("Modell").getAttribute("aria-checked")).toBe("false");
      expect(list.querySelector<HTMLElement>("[data-budget-state]")!.dataset.budgetState).toBe("full");
      expect(button("0 Teile übernehmen")!.disabled).toBe(true);
    });

    it("the_system_dialog_offers_parts_and_models", async () => {
      const w = window as unknown as { __TAURI_INTERNALS__?: unknown };
      w.__TAURI_INTERNALS__ = {};
      try {
        vi.mocked(openFiles).mockResolvedValue(["/p/a.obj"]);
        vi.mocked(inspectMapImport).mockResolvedValue([cube]);
        await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} />));
        await act(async () => button("Datei")!.click());
        await act(async () => button("Dateien wählen…")!.click());
        await flush();
        expect(openFiles).toHaveBeenCalledWith({
          title: "Teile oder Modelle wählen",
          filters: [{ name: "Kartenteile und Modelle", extensions: ["nsbmd", "bmd", "per", "mp", "bld", "bdhc", "bgs", "bin", "obj", "gltf", "glb", "png"] }],
        });
        expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/p/a.obj"] }, { unit: 16, colors: 16 });
      } finally {
        delete w.__TAURI_INTERNALS__;
      }
    });

    it("import_passes_the_options", async () => {
      vi.mocked(inspectMapImport).mockResolvedValue([cube]);
      await showFiles("/p/a.obj");
      const four = [...colours()!.querySelectorAll("button")].find((b) => b.textContent === "4")!;
      await act(async () => four.click());
      await flush();
      await act(async () => button("1 Teil übernehmen")!.click());
      await flush();
      expect(importMapParts).toHaveBeenCalledWith(5, { files: ["/p/a.obj"] }, ["model"], false, { unit: 16, colors: 4 });
    });
  });

  it("shows_a_refused_import_and_reports_no_change", async () => {
    vi.mocked(importMapParts).mockRejectedValue({ code: "error", message: "model: too big", asset: null });
    await show();
    await act(async () => button("2 Teile übernehmen")!.click());
    await flush();
    expect(dialog().querySelector("[role=alert]")?.textContent).toContain("too big");
    expect(onImported).not.toHaveBeenCalled();
  });

  describe("the findings of the map after the import", () => {
    const atMap = (index: number, over: Partial<CheckProblemDto> = {}) =>
      finding("map.model-invalid", "error", { subject: { kind: "map", index }, key: "model", label: `Karte ${String(index).padStart(4, "0")} · Modell`, args: [String(index), "bad magic", "40"], ...over });
    const cell = { kind: "matrixCell" as const, matrix: 0, x: 3, y: 4 };
    const importModel = async () => {
      await show();
      await act(async () => box("Kollision").click());
      await act(async () => button("1 Teil übernehmen")!.click());
      await flush();
    };
    const findings = () => dialog().querySelector<HTMLElement>("[role=list][aria-label='Befunde an Karte 0005']");

    it("waits_for_the_next_report_and_lists_the_findings_of_this_map_with_zeigen", async () => {
      let report!: (f: readonly CheckProblemDto[] | null) => void;
      vi.mocked(checks.nextReport).mockReturnValue(new Promise((resolve) => (report = resolve)));
      await importModel();
      // Asked before the import's edit reaches the checks: the run after its pause counts.
      expect(vi.mocked(checks.nextReport).mock.invocationCallOrder[0]).toBeLessThan(onImported.mock.invocationCallOrder[0]!);
      expect([...dialog().querySelectorAll("[role=status]")].map((e) => e.textContent)).toContain("Prüft die Karte …");
      await act(async () => report([atMap(5, { target: cell }), atMap(6), finding("warp.dest-warp"), atMap(5, { code: "map.bdhc-size", key: "bdhc", args: ["5", "terrain does not start with BDHC", "4"] })]));
      const list = findings()!;
      expect([...list.querySelectorAll<HTMLElement>("[data-finding]")].map((row) => row.dataset.check)).toEqual(["map.model-invalid", "map.bdhc-size"]);
      expect(list.textContent).toContain("Das Modell von Karte 0005 ist ungültig (bad magic). Importiere ein gültiges Modell.");
      expect(list.textContent).toContain("Karte 0005 · Modell");
      // „Zeigen“ only where the finding leads somewhere; it closes the dialog and goes there.
      const zeigen = [...list.querySelectorAll("button")].filter((b) => b.textContent === "Zeigen");
      expect(zeigen).toHaveLength(1);
      await act(async () => zeigen[0]!.click());
      expect(onClose).toHaveBeenCalled();
      expect(goTo).toHaveBeenCalledWith(cell);
    });

    it("says_there_are_no_new_findings_at_this_map", async () => {
      vi.mocked(checks.nextReport).mockResolvedValue([atMap(6)]);
      await importModel();
      expect(dialog().textContent).toContain("Keine neuen Befunde an dieser Karte.");
      expect(findings()).toBeNull();
    });

    it("says_so_when_the_run_gives_no_report", async () => {
      vi.mocked(checks.nextReport).mockResolvedValue(null);
      await importModel();
      expect(dialog().textContent).toContain("Die Prüfungen haben kein Ergebnis geliefert. Starte sie unter Probleme mit „Erneut prüfen“.");
      expect(dialog().textContent).not.toContain("Keine neuen Befunde");
    });

    it("forgets_the_findings_of_the_import_when_the_source_changes", async () => {
      let report!: (f: readonly CheckProblemDto[] | null) => void;
      vi.mocked(checks.nextReport).mockReturnValueOnce(new Promise((resolve) => (report = resolve))).mockResolvedValueOnce([]);
      await importModel();
      expect(dialog().textContent).toContain("Prüft die Karte …");
      // Another source after the import: a fresh check list, nothing of the earlier import.
      const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
      await act(async () => void field.focus());
      typeInto(field, "7");
      await act(async () => void field.blur());
      await flush();
      expect(inspectMapImport).toHaveBeenLastCalledWith(5, { map: 7 });
      expect(dialog().textContent).not.toContain("Prüft die Karte");
      expect(button("2 Teile übernehmen")).toBeDefined();
      // The report of the earlier import arrives late: it is dropped.
      await act(async () => report([atMap(5)]));
      expect(findings()).toBeNull();
      expect(dialog().textContent).not.toContain("Keine neuen Befunde");
      // A report that arrived before the change goes as well.
      await act(async () => button("2 Teile übernehmen")!.click());
      await flush();
      expect(dialog().textContent).toContain("Keine neuen Befunde an dieser Karte.");
      await act(async () => void field.focus());
      typeInto(field, "8");
      await act(async () => void field.blur());
      await flush();
      expect(dialog().textContent).not.toContain("Keine neuen Befunde");
    });

    it("waits_for_nothing_when_the_import_changed_nothing", async () => {
      vi.mocked(importMapParts).mockResolvedValue([]);
      await importModel();
      expect(checks.nextReport).not.toHaveBeenCalled();
      expect(dialog().textContent).toContain("Nichts geändert");
      expect(dialog().textContent).not.toContain("Prüft die Karte");
    });
  });

  describe("a world map", () => {
    const freeform: FreeformInspect = {
      ok: true,
      report: {
        map: 5,
        glb: "0005.haus.glb",
        stats: { polygons: 6, quads: 6, triangles: 0, dropped: 0 },
        textures: [{ name: "g4_imp_cube", file: "/p/g4_imp_cube.png", width: 16, height: 16, colors: 16, new: true, mean_error: 7.6 }],
        notes: [{ code: "model.converted", args: ["glTF", "6", "6", "0", "2", "0"], message: "glTF model converted" }],
        change: null,
        bake: worldBake(),
      },
      refusal: null,
      notes: [],
    };
    const tmj = (over: Partial<TmjImportReport> = {}): TmjImportReport => ({ map: 5, layers: 2, cells: 17, notes: [], errors: [], change: null, bake: worldBake(), ...over });
    const at = (axis: string) => dialog().querySelector<HTMLInputElement>(`input[aria-label='${axis}']`);
    async function showWorldFiles(paths: string) {
      vi.mocked(worldGet).mockResolvedValue(worldDto(true));
      await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} checks={checks} />));
      await flush();
      await act(async () => button("Datei")!.click());
      const area = dialog().querySelector<HTMLTextAreaElement>("textarea")!;
      await act(async () => void area.focus());
      typeInto(area, paths);
      await act(async () => void area.blur());
      await flush();
    }
    async function typeAt(axis: string, text: string) {
      await act(async () => void at(axis)!.focus());
      typeInto(at(axis)!, text);
      await act(async () => void at(axis)!.blur());
      await flush();
    }

    it("a_world_map_offers_freeform_for_glb_obj_and_gltf", async () => {
      vi.mocked(worldFreeformInspect).mockResolvedValue(freeform);
      for (const file of ["/p/haus.glb", "/p/haus.obj", "/p/haus.gltf"]) {
        await act(async () => root.render(<div />));
        await showWorldFiles(file);
        expect(worldGet).toHaveBeenCalledWith(5);
        expect(dialog().querySelector<HTMLElement>("[data-import-dialog]")!.dataset.world).toBe("true");
        expect(worldFreeformInspect).toHaveBeenLastCalledWith({ map: 5, path: file, at: [0, 0, 0], options: { unit: 16, colors: 16 } });
        // The model is no map part of a world map: the part check is not asked.
        expect(inspectMapImport).not.toHaveBeenCalled();
        const row = dialog().querySelector<HTMLElement>("[data-import-part='freeform']")!;
        expect(row.textContent).toContain("Freiform (glTF)");
        expect(row.textContent).toContain("glTF-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 2 Streifen.");
        expect(row.textContent).toContain("g4_imp_cube · 16 × 16 · 16 Farben · neu");
        expect(row.querySelector("[role=meter]")!.getAttribute("aria-valuetext")).toBe("21.740 von 61.440 B");
        expect(box("Freiform (glTF)").getAttribute("aria-checked")).toBe("true");
        // „Lage“ above the row: three number fields in world units.
        const place = dialog().querySelector<HTMLElement>("[data-import-at]")!;
        expect(place.textContent).toContain("Lage");
        expect(place.textContent).toContain("16 = eine Kachel; 0, 0, 0 = Kartenmitte");
        expect([at("x")!.value, at("y")!.value, at("z")!.value]).toEqual(["0", "0", "0"]);
        expect(place.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
        expect(button("1 Teil übernehmen")!.disabled).toBe(false);
      }
      // A refused model says why and cannot be chosen.
      vi.mocked(worldFreeformInspect).mockResolvedValue({
        ok: false,
        report: null,
        refusal: { code: "model.no-geometry", args: [], message: "no faces" },
        notes: [],
      });
      await act(async () => root.render(<div />));
      await showWorldFiles("/p/leer.obj");
      const row = dialog().querySelector<HTMLElement>("[data-import-part='freeform']")!;
      expect(row.textContent).toContain("Das Modell enthält keine Flächen.");
      expect(box("Freiform (glTF)").getAttribute("aria-checked")).toBe("false");
      expect(button("0 Teile übernehmen")!.disabled).toBe(true);
    });

    it("freeform_sends_at_and_options", async () => {
      vi.mocked(worldFreeformInspect).mockResolvedValue(freeform);
      vi.mocked(worldFreeformAdd).mockResolvedValue({ ...freeform.report!, change: 9 });
      await showWorldFiles("/p/haus.glb");
      await typeAt("x", "16");
      await typeAt("z", "-32");
      expect(worldFreeformInspect).toHaveBeenLastCalledWith({ map: 5, path: "/p/haus.glb", at: [16, 0, -32], options: { unit: 16, colors: 16 } });
      // A field that is no whole number says so and asks nothing.
      const calls = vi.mocked(worldFreeformInspect).mock.calls.length;
      await typeAt("y", "abc");
      expect(dialog().textContent).toContain("Das ist keine ganze Zahl.");
      expect(worldFreeformInspect).toHaveBeenCalledTimes(calls);
      await typeAt("y", "8");
      const four = [...dialog().querySelectorAll<HTMLElement>("[aria-label='Farben je Textur'] button")].find((b) => b.textContent === "4")!;
      await act(async () => four.click());
      await flush();
      expect(worldFreeformInspect).toHaveBeenLastCalledWith({ map: 5, path: "/p/haus.glb", at: [16, 8, -32], options: { unit: 16, colors: 4 } });
      await act(async () => button("1 Teil übernehmen")!.click());
      await flush();
      expect(worldFreeformAdd).toHaveBeenCalledExactlyOnceWith({ map: 5, path: "/p/haus.glb", at: [16, 8, -32], options: { unit: 16, colors: 4 } });
      expect(importMapParts).not.toHaveBeenCalled();
      expect(onImported).toHaveBeenCalledWith(5);
      expect(dialog().textContent).toContain("1 Teil übernommen");
    });

    it("lage_takes_only_the_bounds_of_the_world_schema", async () => {
      vi.mocked(worldFreeformInspect).mockResolvedValue(freeform);
      await showWorldFiles("/p/haus.glb");
      const calls = vi.mocked(worldFreeformInspect).mock.calls.length;
      // −512 to 511 per axis, like `at` of a free-form entry in the world.
      await typeAt("x", "512");
      expect(dialog().textContent).toContain("höchstens 511");
      await typeAt("z", "-513");
      expect(dialog().textContent).toContain("mindestens -512");
      expect(worldFreeformInspect).toHaveBeenCalledTimes(calls);
      await typeAt("x", "511");
      await typeAt("z", "-512");
      expect(worldFreeformInspect).toHaveBeenLastCalledWith({ map: 5, path: "/p/haus.glb", at: [511, 0, -512], options: { unit: 16, colors: 16 } });
    });

    it("a_failing_tmj_after_the_freeform_keeps_the_freeform_taken_and_announces_it", async () => {
      vi.mocked(worldFreeformInspect).mockResolvedValue(freeform);
      vi.mocked(worldFreeformAdd).mockResolvedValue({ ...freeform.report!, change: 9 });
      vi.mocked(worldInspectTmj).mockResolvedValue(tmj());
      vi.mocked(worldImportTmj).mockRejectedValueOnce(new Error("Die Karte ist gesperrt."));
      await showWorldFiles("/p/haus.obj\n/p/0005.tmj");
      expect(button("2 Teile übernehmen")!.disabled).toBe(false);
      await act(async () => button("2 Teile übernehmen")!.click());
      await flush();
      expect(worldFreeformAdd).toHaveBeenCalledTimes(1);
      expect(dialog().textContent).toContain("Die Karte ist gesperrt.");
      // The free-form went in: the scene and the inspector hear of it.
      expect(onImported).toHaveBeenCalledExactlyOnceWith(5);
      // It is no longer chosen: a retry imports only the TMJ, not the free-form a second time.
      expect(box("Freiform (glTF)").getAttribute("aria-checked")).toBe("false");
      vi.mocked(worldImportTmj).mockResolvedValue(tmj({ change: 10 }));
      await act(async () => button("1 Teil übernehmen")!.click());
      await flush();
      expect(worldFreeformAdd).toHaveBeenCalledTimes(1);
      expect(worldImportTmj).toHaveBeenCalledTimes(2);
      expect(onImported).toHaveBeenCalledTimes(2);
    });

    it("tmj_shows_changes_or_errors", async () => {
      vi.mocked(worldInspectTmj).mockResolvedValue(tmj({ notes: ["tmj.buildings-ignored"] }));
      vi.mocked(worldImportTmj).mockResolvedValue(tmj({ change: 4 }));
      await showWorldFiles("/p/0005.tmj");
      expect(worldInspectTmj).toHaveBeenLastCalledWith(5, "/p/0005.tmj");
      expect(inspectMapImport).not.toHaveBeenCalled();
      // TMJ is not converted: no „Umwandeln“, no „Lage“.
      expect(dialog().querySelector("[data-import-convert]")).toBeNull();
      expect(dialog().querySelector("[data-import-at]")).toBeNull();
      let row = dialog().querySelector<HTMLElement>("[data-import-part='tmj']")!;
      expect(row.textContent).toContain("Tiled (TMJ)");
      expect(row.textContent).toContain("2 Ebenen, 17 Zellen geändert");
      expect(row.textContent).toContain("Gebäude");
      expect(row.textContent).not.toContain("tmj.buildings-ignored");
      expect(box("Tiled (TMJ)").getAttribute("aria-checked")).toBe("true");
      await act(async () => button("1 Teil übernehmen")!.click());
      await flush();
      expect(worldImportTmj).toHaveBeenCalledExactlyOnceWith(5, "/p/0005.tmj");
      expect(onImported).toHaveBeenCalledWith(5);
      expect(dialog().textContent).toContain("1 Teil übernommen");

      // The same world: nothing to take over.
      vi.mocked(worldInspectTmj).mockResolvedValue(tmj({ cells: 0, bake: null }));
      await act(async () => root.render(<div />));
      await showWorldFiles("/p/0005.tmj");
      row = dialog().querySelector<HTMLElement>("[data-import-part='tmj']")!;
      expect(row.textContent).toContain("Keine Änderung.");
      expect(box("Tiled (TMJ)").getAttribute("aria-checked")).toBe("false");
      expect(button("0 Teile übernehmen")!.disabled).toBe(true);

      // Errors: the first three with their paths, then how many more.
      const errors = Array.from({ length: 5 }, (_, i) => ({ path: `layers[0].data[${i}]`, message: `GID ${500 + i} gehört nicht in diese Ebene.` }));
      vi.mocked(worldInspectTmj).mockResolvedValue(tmj({ layers: 0, cells: 0, bake: null, errors }));
      await act(async () => root.render(<div />));
      await showWorldFiles("/p/0005.tmj");
      row = dialog().querySelector<HTMLElement>("[data-import-part='tmj']")!;
      expect(row.textContent).toContain("layers[0].data[0]: GID 500 gehört nicht in diese Ebene.");
      expect(row.textContent).toContain("layers[0].data[2]: GID 502 gehört nicht in diese Ebene.");
      expect(row.textContent).not.toContain("layers[0].data[3]");
      expect(row.textContent).toContain("… und 2 weitere");
      expect(box("Tiled (TMJ)").getAttribute("aria-checked")).toBe("false");
      expect(button("0 Teile übernehmen")!.disabled).toBe(true);
    });

    it("other_parts_are_disabled_for_world_maps", async () => {
      vi.mocked(worldGet).mockResolvedValue(worldDto(true));
      await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} checks={checks} />));
      await flush();
      const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
      await act(async () => void field.focus());
      typeInto(field, "6");
      await act(async () => void field.blur());
      await flush();
      expect(dialog().textContent).toContain("Bei Welt-Karten entstehen Modell, Terrain und Permissions beim Backen.");
      for (const name of ["Modell", "Kollision", "Gebäude"]) {
        expect(box(name).getAttribute("aria-checked"), name).toBe("false");
        expect(box(name).getAttribute("aria-disabled") === "true" || box(name).hasAttribute("data-disabled"), name).toBe(true);
      }
      expect(button("0 Teile übernehmen")!.disabled).toBe(true);
      // A binary map keeps its parts.
      vi.mocked(worldGet).mockResolvedValue(worldDto(false));
      await act(async () => root.render(<div />));
      await show();
      expect(dialog().querySelector<HTMLElement>("[data-import-dialog]")!.dataset.world).toBe("false");
      expect(dialog().textContent).not.toContain("Bei Welt-Karten");
      expect(button("2 Teile übernehmen")).toBeDefined();
    });
  });
});
