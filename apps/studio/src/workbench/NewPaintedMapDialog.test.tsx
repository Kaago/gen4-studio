// @vitest-environment jsdom
import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { recipeApply, recipePlan, tilesetGet } from "../api";
import type { DraftDto, MatrixDto, NewPaintedMapReport, RecipeOutcome, TileJson, TilesetDto } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { typeInto } from "../ui/primitives/testUtils";
import { visibleWords } from "../ui/visibleText";
import { resetSectionFolds } from "./inspector/foldState";
import { NewPaintedMapDialog } from "./NewPaintedMapDialog";
import { NO_WORLD, WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";
import { DRAFT_DEBOUNCE_MS } from "./wizards/useDraftPlan";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), recipePlan: vi.fn(), recipeApply: vi.fn(), tilesetGet: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const matrix: MatrixDto = { index: 0, name: "m", width: 4, height: 1, maps: [1, 2, null, null], headers: [411, 411, 0, 0] };

const tile = (texture: string, role: TileJson["role"]): TileJson => ({ texture, texture_index: 0, size: [32, 32], role, kind: 0, collision: 0, height: "flat", origin: "vanilla" });
const TILESET: TilesetDto = { package: 6, stored: true, tileset: { version: 1, package: 6, tiles: [tile("weg", "ground"), tile("gras_a", "grass")] } };

// The plan of the painted map as the kernel answers it.
const planned: DraftDto = {
  cells: [{ matrix: 0, x: 2, y: 0, source_map: 2 }],
  tiles: [],
  events: [],
  messages: [],
  notes: ["Neue Karte „Waldweg“ in Zelle x 2, y 0 der Weltkarte 0000.", "Boden aus Tileset 0006, Kachel 0000, Höhe 0."],
  irreversible: ["Karte 0667 wird angelegt"],
  painted: { tileset: 6, fill_tile: "0000", height: 0, header: 411 },
};

const report: NewPaintedMapReport = { map: 667, name: "Waldweg", matrix: 0, x: 2, y: 0, width: 4, height: 1, header: 411, tileset: 6, base: 2, bake: null };

describe("NewPaintedMapDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onDone = vi.fn();
  const onError = vi.fn();
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  const startPainting = vi.fn();
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    vi.mocked(recipeApply).mockReset();
    vi.mocked(tilesetGet).mockReset().mockResolvedValue(TILESET);
    for (const f of [onDone, onError, onDraft, startPainting]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  const inWorkbench = (children: ReactNode) => {
    const data = { startPainting } as unknown as WorkbenchData;
    return (
      <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world: NO_WORLD }}>
        {children}
      </WorkbenchProviders>
    );
  };
  const show = (depth: Depth = "tuefteln") =>
    act(() =>
      root.render(
        inWorkbench(<NewPaintedMapDialog matrix={matrix} x={2} y={0} depth={depth} onDone={onDone} onError={onError} onDraft={onDraft} image={async () => null} />),
      ),
    );
  const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text);
  const field = (name: string) => dialog().querySelector<HTMLInputElement>(`input[aria-label^="${name}"]`);
  const commit = async (name: string, text: string) => {
    await act(async () => field(name)!.focus());
    typeInto(field(name)!, text);
    await act(async () => field(name)!.blur());
  };

  it("plans_with_the_cell_and_shows_the_draft", async () => {
    show();
    expect(dialog().textContent).toContain("Neue Karte malen");
    await commit("Name", "Waldweg");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", height: 0 });
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    // The notes of the draft and the badge with what undo would not take back.
    expect(dialog().querySelector("[data-draft-notes]")?.textContent).toContain("Neue Karte „Waldweg“ in Zelle x 2, y 0 der Weltkarte 0000.");
    expect(dialog().textContent).toContain("Nicht rückgängigKarte 0667 wird angelegt");
    // „Boden“: the palette of the draft's package, the draft's tile pressed.
    expect(tilesetGet).toHaveBeenCalledWith(6);
    await settle();
    const ground = dialog().querySelector("[data-paint-ground]")!;
    expect(ground.querySelector("[data-palette-tile='0000']")?.getAttribute("aria-pressed")).toBe("true");
    await act(async () => ground.querySelector<HTMLElement>("[data-palette-tile='0001']")!.click());
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", fill_tile: "0001", height: 0 });
    expect(ground.querySelector("[data-palette-tile='0001']")?.getAttribute("aria-pressed")).toBe("true");
    // „Höhe“ in steps.
    expect(field("Höhe")?.value).toBe("0");
    await commit("Höhe", "2");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", fill_tile: "0001", height: 2 });
  });

  it("apply_reports_the_map_and_starts_painting", async () => {
    const outcome: RecipeOutcome<NewPaintedMapReport> = { report, created: [{ kind: "Map", index: 667 }], change: 9, changed: [{ kind: "Matrix", index: 0 }], next: [] };
    vi.mocked(recipeApply).mockResolvedValue(outcome);
    show();
    await commit("Name", "Waldweg");
    await settle();
    await act(async () => button("Karte malen")?.click());
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", height: 0 });
    expect(onDone).toHaveBeenCalledExactlyOnceWith(report);
    expect(startPainting).toHaveBeenCalledExactlyOnceWith({ matrix: 0, x: 2, y: 0 });
    expect(onError).not.toHaveBeenCalled();
  });

  it("name_is_required", async () => {
    show();
    await settle();
    // Without a name nothing is planned and „Karte malen“ stays locked.
    expect(recipePlan).not.toHaveBeenCalled();
    expect(dialog().textContent).toContain("Pflicht");
    expect(button("Karte malen")?.disabled).toBe(true);
    await commit("Name", "   ");
    await settle();
    expect(recipePlan).not.toHaveBeenCalled();
    expect(button("Karte malen")?.disabled).toBe(true);
    await commit("Name", "Malweg");
    await settle();
    expect(recipePlan).toHaveBeenCalledOnce();
    expect(button("Karte malen")?.disabled).toBe(false);
  });

  it("the_name_counts_while_typing", async () => {
    vi.mocked(recipeApply).mockResolvedValue({ report, created: [], change: 9, changed: [], next: [] });
    show();
    // Typed, never left: „Karte malen“ is free at once and the plan follows (debounced).
    await act(async () => field("Name")!.focus());
    typeInto(field("Name")!, "Malweg");
    expect(button("Karte malen")?.disabled).toBe(false);
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Malweg", height: 0 });
    await act(async () => button("Karte malen")?.click());
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Malweg", height: 0 });
  });

  it("tinkering_offers_tileset_and_header", async () => {
    show("bauen");
    await commit("Name", "Waldweg");
    await settle();
    expect(field("Tileset")).toBeNull();
    expect(field("Header")).toBeNull();
    expect(visibleWords(dialog())).not.toMatch(/Tileset|Header/);
    act(() => root.render(null));
    show("tuefteln");
    await commit("Name", "Waldweg");
    await settle();
    // The draft's values until the user sets one.
    expect(field("Tileset")?.value).toBe("6");
    expect(field("Header")?.value).toBe("411");
    await commit("Tileset", "2");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", tileset: 2, height: 0 });
    await commit("Header", "412");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_painted_map", { matrix: 0, x: 2, y: 0, name: "Waldweg", tileset: 2, height: 0, header: 412 });
  });
});
