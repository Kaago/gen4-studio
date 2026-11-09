// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMap, getMapBudget, getMatrix, tileBehaviors, tilesetEdit, tilesetSave, worldBake } from "../../api";
import type { MatrixDto } from "../../api";
import type { EdgePlace, EdgeSetJson, TileJson, TilesetDto, WorldGetDto } from "../../api.world";
import type { Depth } from "../../ui/frame/shellState";
import { typeInto } from "../../ui/primitives/testUtils";
import { INITIAL_PAINT, PaintContext } from "../paint/PaintContext";
import type { PaintModel, PaintPatch, PaintState } from "../paint/PaintContext";
import { EdgeSetEditor } from "./EdgeSetEditor";
import { resetSectionFolds } from "./foldState";
import { MatrixCellEditor } from "./MatrixCellEditor";
import type { MatrixCellContext } from "./MatrixCellEditor";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  tilesetEdit: vi.fn(),
  tilesetSave: vi.fn(),
  getMatrix: vi.fn(),
  getMapBudget: vi.fn(),
  getMap: vi.fn(),
  worldBake: vi.fn(),
  tileBehaviors: vi.fn(),
}));
// The pictures: none (placeholders), no kernel asked.
vi.mock("../../world/paint/paletteImages", () => ({ tileImage: async () => null, peekTileImage: () => undefined, resetTileImages: () => {}, tileImagesGeneration: () => 0, onTileImagesReset: () => () => {} }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const tile = (texture: string): TileJson => ({ texture, texture_index: 0, size: [32, 32], role: "ground", kind: 0, collision: 0, height: "flat", origin: "vanilla" });
/** Package 0 with 16 ground tiles; „Weg“ fills the middle with 0006, edge N with 0007r1 and outer NW with two variants. */
const WEG: EdgeSetJson = { name: "Weg", outside: "0000", places: { center: ["0006"], n: ["0007r1"], outer_nw: ["0008", "0009f"] } };
const FELS: EdgeSetJson = { name: "Fels", outside: "0001", block: "lower", places: {} };
const tileset = (stored = true, sets: EdgeSetJson[] = [WEG, FELS]): TilesetDto => ({
  package: 0,
  stored,
  tileset: { version: 1, package: 0, tiles: Array.from({ length: 16 }, (_, i) => tile(`t${i}`)), edge_sets: sets },
});

describe("EdgeSetEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onActive = vi.fn();
  const onPlace = vi.fn();
  const onChanged = vi.fn();
  const onError = vi.fn();
  const onRise = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    vi.mocked(tilesetEdit).mockReset().mockResolvedValue({ package: 0, tiles: 0, change: 1 });
    vi.mocked(tilesetSave).mockReset().mockResolvedValue({ package: 0, file: "data/tilesets/0000.json", tiles: 16, vanilla: 16, unclear: 0, change: 1 });
    for (const fn of [onActive, onPlace, onChanged, onError, onRise]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(over: { set?: TilesetDto; active?: string | null; place?: EdgePlace | null; tile?: string; depth?: Depth } = {}) {
    await act(async () =>
      root.render(
        <EdgeSetEditor
          tileset={over.set ?? tileset()}
          active={over.active === undefined ? "Weg" : over.active}
          place={over.place ?? null}
          tile={over.tile ?? "0002"}
          rise={2}
          onRise={onRise}
          onActive={onActive}
          onPlace={onPlace}
          onChanged={onChanged}
          onError={onError}
          depth={over.depth ?? "bauen"}
        />,
      ),
    );
    await flush();
  }
  const place = (p: EdgePlace) => container.querySelector<HTMLButtonElement>(`[data-edge-place="${p}"]`)!;
  const button = (text: string, within: ParentNode = container) => [...within.querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;

  it("lists_the_sets_with_their_counts_and_the_places_of_the_active_one", async () => {
    await show();
    const rows = container.querySelectorAll("[data-edge-sets] [data-edge-set]");
    expect([...rows].map((r) => r.getAttribute("data-edge-set"))).toEqual(["Weg", "Fels"]);
    const weg = container.querySelector('[data-edge-set="Weg"]')!;
    expect(weg.textContent).toContain("3 von 13 Plätzen · Diagonalen 0 von 2");
    expect(weg.querySelector<HTMLInputElement>('input[type="radio"][aria-label="Aktiv"]')!.checked).toBe(true);
    expect(container.querySelector<HTMLInputElement>('[data-edge-set="Fels"] input[type="radio"]')!.checked).toBe(false);
    const editor = container.querySelector("[data-edge-editor]")!;
    expect(editor.querySelectorAll("[data-edge-place]")).toHaveLength(15);
    expect(place("outer_nw").getAttribute("aria-label")).toBe("Ecke außen NW, 2 Kacheln");
    expect(place("n").getAttribute("aria-label")).toBe("Kante N, 1 Kachel");
    expect(place("inner_se").getAttribute("aria-label")).toBe("Ecke innen SO, keine Kachel");
    expect(place("inner_se").hasAttribute("data-edge-empty")).toBe(true);
    // A small counter only with more than one variant.
    expect(place("outer_nw").querySelector("[data-edge-count]")?.textContent).toBe("2");
    expect(place("n").querySelector("[data-edge-count]")).toBeNull();
    // Bauen shows no tokens; Tüfteln adds them.
    expect(place("n").parentElement!.textContent).not.toContain("0007r1");
    await show({ depth: "tuefteln" });
    expect(place("n").parentElement!.textContent).toContain("0007r1");
  });

  it("clicking_a_place_selects_it", async () => {
    await show();
    expect(place("n").getAttribute("aria-pressed")).toBe("false");
    act(() => place("n").click());
    expect(onPlace).toHaveBeenCalledExactlyOnceWith("n");
    await show({ place: "outer_nw" });
    expect(place("outer_nw").getAttribute("aria-pressed")).toBe("true");
    expect([...container.querySelectorAll("[data-edge-variant]")].map((c) => c.getAttribute("data-edge-variant"))).toEqual(["0008", "0009f"]);
    // The chosen place again: none chosen.
    act(() => place("outer_nw").click());
    expect(onPlace).toHaveBeenLastCalledWith(null);
    expect(tilesetEdit).not.toHaveBeenCalled();
  });

  it("removing_a_variant_is_one_edit", async () => {
    await show({ place: "outer_nw" });
    const chip = container.querySelector('[data-edge-variant="0009f"]')!;
    act(() => button("Entfernen", chip).click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { ...WEG, places: { ...WEG.places, outer_nw: ["0008"] } } }]);
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("block_lower_is_one_edit", async () => {
    await show();
    const block = container.querySelector("[data-edge-block]")!;
    expect(block.querySelector('[aria-pressed="true"]')?.textContent).toBe("Obere Zelle");
    act(() => button("Untere Zelle", block).click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { ...WEG, block: "lower" } }]);
    // Back to the upper cell: the default, left out.
    vi.mocked(tilesetEdit).mockClear();
    await show({ active: "Fels" });
    act(() => button("Obere Zelle", container.querySelector("[data-edge-block]")!).click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { name: "Fels", outside: "0001", places: {} } }]);
  });

  it("outside_takes_the_active_tile", async () => {
    await show({ tile: "0003r2" });
    act(() => container.querySelector<HTMLButtonElement>("[data-edge-outside]")!.click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { ...WEG, outside: "0003r2" } }]);
  });

  it("wall_sets_every_tile_of_the_set_in_the_same_edit", async () => {
    await show();
    const wall = container.querySelector("[data-edge-wall] input")! as HTMLInputElement;
    await act(async () => void wall.focus());
    typeInto(wall, "000f");
    await act(async () => void wall.blur());
    await flush();
    // Ruling 13: every tile of the set, the outside (0000) and the places (0006, 0007, 0008, 0009), in one step with the set.
    expect(container.querySelector("[data-edge-wall]")!.textContent).toContain("Für jede Kachel des Satzes, auch Außen");
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(
      0,
      [0, 6, 7, 8, 9].map((t) => ({ tile: t, wall: 15 })),
      [{ put: WEG }],
    );
    expect(onChanged).toHaveBeenCalledOnce();
    // A tile past the tileset is refused, nothing written.
    vi.mocked(tilesetEdit).mockClear();
    await act(async () => void wall.focus());
    typeInto(wall, "0010");
    await act(async () => void wall.blur());
    await flush();
    expect(tilesetEdit).not.toHaveBeenCalled();
    expect(container.querySelector("[data-edge-wall]")!.textContent).toContain("Eine Kachel von 0000 bis 000f.");
  });

  it("the_wall_shows_the_common_wall_and_empties_to_none", async () => {
    const set = tileset();
    set.tileset.tiles = set.tileset.tiles.map((t, i) => (i === 0 || (i >= 6 && i <= 9) ? { ...t, wall: 12 } : t));
    await show({ set });
    const wall = container.querySelector("[data-edge-wall] input")! as HTMLInputElement;
    expect(wall.value).toBe("000c");
    await act(async () => void wall.focus());
    typeInto(wall, "");
    await act(async () => void wall.blur());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [0, 6, 7, 8, 9].map((t) => ({ tile: t, wall: null })), [{ put: WEG }]);
  });

  it("the_common_wall_counts_the_outside_tile", async () => {
    // The places share wall 12, the outside has none: no common wall.
    const set = tileset();
    set.tileset.tiles = set.tileset.tiles.map((t, i) => (i >= 6 && i <= 9 ? { ...t, wall: 12 } : t));
    await show({ set });
    expect((container.querySelector("[data-edge-wall] input") as HTMLInputElement).value).toBe("");
  });

  it("new_set_takes_the_active_tile_as_outside", async () => {
    await show({ tile: "0002r1" });
    expect(input("Name des neuen Kantensatzes").value).toBe("Satz 3");
    act(() => button("Neuer Kantensatz").click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { name: "Satz 3", outside: "0002r1", places: {} } }]);
    expect(onActive).toHaveBeenCalledExactlyOnceWith("Satz 3");
    expect(onChanged).toHaveBeenCalledOnce();
    // A name the tileset has already is refused before the kernel (a put would replace that set).
    vi.mocked(tilesetEdit).mockClear();
    const name = input("Name des neuen Kantensatzes");
    await act(async () => void name.focus());
    typeInto(name, "Weg");
    await act(async () => void name.blur());
    act(() => button("Neuer Kantensatz").click());
    await flush();
    expect(tilesetEdit).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Einen Kantensatz „Weg“ gibt es schon.");
  });

  it("removing_the_set_is_one_edit_and_leaves_no_active_set", async () => {
    await show({ place: "n" });
    act(() => button("Satz entfernen").click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ remove: { name: "Weg" } }]);
    expect(onActive).toHaveBeenCalledWith(null);
    expect(onPlace).toHaveBeenCalledWith(null);
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("a_refusal_goes_to_onError", async () => {
    const refusal = new Error("Schema");
    vi.mocked(tilesetEdit).mockRejectedValueOnce(refusal);
    await show();
    act(() => button("Untere Zelle", container.querySelector("[data-edge-block]")!).click());
    await flush();
    expect(onError).toHaveBeenCalledExactlyOnceWith(refusal);
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("the_cliff_steps_stay_within_one_to_eight", async () => {
    await show();
    const rise = container.querySelector("[data-cliff-rise] input")! as HTMLInputElement;
    expect(rise.value).toBe("2");
    await act(async () => void rise.focus());
    typeInto(rise, "5");
    await act(async () => void rise.blur());
    expect(onRise).toHaveBeenCalledExactlyOnceWith(5);
    onRise.mockClear();
    for (const text of ["9", "0"]) {
      await act(async () => void rise.focus());
      typeInto(rise, text);
      await act(async () => void rise.blur());
    }
    expect(onRise).not.toHaveBeenCalled();
    expect(tilesetEdit).not.toHaveBeenCalled();
  });

  it("without_file_the_editor_waits_for_tileset_save", async () => {
    await show({ set: tileset(false) });
    expect(button("Neuer Kantensatz").disabled).toBe(true);
    expect(container.querySelector<HTMLButtonElement>("[data-edge-outside]")!.disabled).toBe(true);
    expect(button("Satz entfernen").disabled).toBe(true);
    expect((container.querySelector("[data-edge-wall] input") as HTMLInputElement).disabled).toBe(true);
    act(() => container.querySelector<HTMLButtonElement>("[data-edge-sets] [data-tileset-save]")!.click());
    await flush();
    expect(tilesetSave).toHaveBeenCalledExactlyOnceWith(0);
    expect(onChanged).toHaveBeenCalledOnce();
    expect(tilesetEdit).not.toHaveBeenCalled();
  });

  it("without_sets_it_says_how_to_start", async () => {
    await show({ set: tileset(true, []), active: null });
    expect(container.querySelector("[data-edge-editor]")).toBeNull();
    expect(container.querySelector("[data-edge-sets]")!.textContent).toContain("Noch kein Kantensatz");
  });

  describe("in the cell editor", () => {
    const world: WorldGetDto = {
      map: 11,
      world: { version: 1, tileset: 0, layers: [{ name: "Boden", role: "ground", tiles: [] }], permissions: { derive: true } },
      faces: 1024,
      textures: 2,
      estimate: 30000,
      share: 0.5,
      bake: null,
    };
    const matrix: MatrixDto = { index: 3, name: "m", width: 2, height: 1, maps: [10, 11], headers: [411, 412], altitudes: [0, 2] };
    const context = (): MatrixCellContext => ({
      dirty: [],
      headers: [],
      identify: () => ({ kind: "area", name: "Zweiblattdorf", place: "Zweiblattdorf" }),
      headerCount: 500,
      goTo: vi.fn(),
      counts: { Map: 600 },
      revision: 0,
      onEdited: vi.fn(),
      onCountsChanged: vi.fn(),
      onError,
      onMapImported: onChanged,
      checks: null,
      painting: true,
    });
    let set: ReturnType<typeof vi.fn<(patch: PaintPatch) => void>>;
    async function showCell(state: Partial<PaintState>, sets: EdgeSetJson[] = [{ ...WEG, places: { center: ["0006"] } }, FELS]) {
      vi.mocked(getMatrix).mockReset().mockResolvedValue(matrix);
      vi.mocked(getMap).mockReset().mockResolvedValue({ world: true } as never);
      vi.mocked(worldBake).mockReset().mockResolvedValue(null);
      vi.mocked(tileBehaviors).mockReset().mockResolvedValue([]);
      vi.mocked(getMapBudget).mockReset().mockImplementation(async (map) => ({
        map,
        model: { used: 1, max: 61440, state: "calm" },
        bdhc: { used: 1, max: 36864, state: "calm" },
        buildings: { used: 0, max: 32, state: "calm" },
        textures: null,
        textures_missing: "",
      }));
      set = vi.fn<(patch: PaintPatch) => void>();
      const model: PaintModel = { state: { ...INITIAL_PAINT, ...state }, set, target: 11, world, tileset: tileset(true, sets), hidden: new Set(), toggleHidden: vi.fn() };
      await act(async () =>
        root.render(
          <PaintContext.Provider value={model}>
            <MatrixCellEditor matrix={3} x={1} y={0} context={context()} depth="bauen" />
          </PaintContext.Provider>,
        ),
      );
      await flush();
    }

    it("a_palette_click_adds_the_turned_token_to_the_place", async () => {
      // Z turned the active tile a quarter („0003r1“); place N chosen, then tile 0007 in the palette.
      await showCell({ sub: "terrain", tile: "0003r1", edgeSet: "Weg" });
      expect(container.querySelector("[data-paint-palette] [data-palette-target]")).toBeNull();
      act(() => place("n").click());
      expect(place("n").getAttribute("aria-pressed")).toBe("true");
      expect(container.querySelector("[data-paint-palette] [data-palette-target]")?.textContent).toContain("„Kante N“");
      act(() => container.querySelector<HTMLButtonElement>('[data-palette-tile="0007"]')!.click());
      await flush();
      expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(0, [], [{ put: { ...WEG, places: { center: ["0006"], n: ["0007r1"] } } }]);
      expect(onChanged).toHaveBeenCalledExactlyOnceWith(11);
      // The palette shows the token added; the terrain brush stays.
      expect(set).toHaveBeenCalledExactlyOnceWith({ tile: "0007r1" });
      expect(onError).not.toHaveBeenCalled();
    });

    it("without_a_chosen_place_a_palette_click_only_chooses_the_tile", async () => {
      await showCell({ sub: "brush", tile: "0003r1", edgeSet: "Weg" });
      act(() => container.querySelector<HTMLButtonElement>('[data-palette-tile="0007"]')!.click());
      await flush();
      expect(tilesetEdit).not.toHaveBeenCalled();
      expect(set).toHaveBeenCalledOnce();
      // 6.4a: the plain tile, and from a tool that paints no tile back to the brush.
      const patch = set.mock.calls[0][0];
      expect(typeof patch === "function" ? patch({ ...INITIAL_PAINT, sub: "terrain" }) : patch).toEqual({ tile: "0007", sub: "brush" });
      expect(typeof patch === "function" ? patch({ ...INITIAL_PAINT, sub: "fill" }) : patch).toEqual({ tile: "0007" });
    });

    const palette = (digits: string) => container.querySelector<HTMLButtonElement>(`[data-palette-tile="${digits}"]`)!;

    it("a_quick_second_palette_click_keeps_the_first_tile", async () => {
      let answer: (v: { package: number; tiles: number; change: number }) => void = () => {};
      vi.mocked(tilesetEdit).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
      await showCell({ sub: "terrain", tile: "0003r1", edgeSet: "Weg" });
      act(() => place("n").click());
      act(() => palette("0007").click());
      // While the first append runs, a second click does nothing, not even in the palette.
      act(() => palette("0008").click());
      expect(tilesetEdit).toHaveBeenCalledOnce();
      expect(set).toHaveBeenCalledOnce();
      await act(async () => answer({ package: 0, tiles: 0, change: 1 }));
      await flush();
      // The tileset is not read again yet: the next append builds on the set written, 0007r1 stays.
      act(() => palette("0008").click());
      await flush();
      expect(tilesetEdit).toHaveBeenCalledTimes(2);
      expect(vi.mocked(tilesetEdit).mock.calls[1]).toEqual([0, [], [{ put: { ...WEG, places: { center: ["0006"], n: ["0007r1", "0008r1"] } } }]]);
      expect(onError).not.toHaveBeenCalled();
    });

    it("a_chip_removed_after_an_append_keeps_the_append", async () => {
      await showCell({ sub: "terrain", tile: "0007r1", edgeSet: "Weg" }, [{ ...WEG, places: { center: ["0006", "000a"] } }]);
      act(() => place("n").click());
      act(() => palette("0007").click());
      await flush();
      act(() => place("center").click());
      expect([...container.querySelectorAll("[data-edge-variant]")].map((c) => c.getAttribute("data-edge-variant"))).toEqual(["0006", "000a"]);
      act(() => button("Entfernen", container.querySelector('[data-edge-variant="000a"]')!).click());
      await flush();
      expect(tilesetEdit).toHaveBeenCalledTimes(2);
      expect(vi.mocked(tilesetEdit).mock.calls[1]).toEqual([0, [], [{ put: { ...WEG, places: { center: ["0006"], n: ["0007r1"] } } }]]);
      // The place shows the append before the tileset is read again.
      expect(place("n").getAttribute("aria-label")).toBe("Kante N, 1 Kachel");
    });

    it("a_failing_refresh_after_an_append_goes_to_onError", async () => {
      const failure = new Error("refetch");
      onChanged.mockRejectedValueOnce(failure);
      await showCell({ sub: "terrain", tile: "0007", edgeSet: "Weg" });
      act(() => place("n").click());
      act(() => palette("0007").click());
      await flush();
      expect(onError).toHaveBeenCalledExactlyOnceWith(failure);
    });

    it("the_active_set_goes_to_the_paint_state", async () => {
      await showCell({ sub: "terrain", edgeSet: null });
      expect(container.querySelector("[data-edge-editor]")).toBeNull();
      act(() => container.querySelector<HTMLInputElement>('[data-edge-set="Fels"] input[type="radio"]')!.click());
      expect(set).toHaveBeenCalledExactlyOnceWith({ edgeSet: "Fels" });
      // The cliff's steps go there too.
      set.mockClear();
      const rise = container.querySelector("[data-cliff-rise] input")! as HTMLInputElement;
      await act(async () => void rise.focus());
      typeInto(rise, "4");
      await act(async () => void rise.blur());
      expect(set).toHaveBeenCalledExactlyOnceWith({ cliffRise: 4 });
    });
  });
});
