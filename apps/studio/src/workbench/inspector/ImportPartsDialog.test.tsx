// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { importMapParts, inspectMapImport } from "../../api";
import type { CheckProblemDto, PartReport } from "../../api";
import { fakeChecks, finding } from "../../checks/checksTesting";
import type { ChecksApi } from "../../checks/useChecks";
import { typeInto } from "../../ui/primitives/testUtils";
import { ImportPartsDialog } from "./ImportPartsDialog";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), inspectMapImport: vi.fn(), importMapParts: vi.fn() }));

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
});
