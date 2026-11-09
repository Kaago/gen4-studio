// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TileJson, TileRole, TilesetDto } from "../../api.world";
import type { Depth } from "../../ui/frame/shellState";
import { resetSectionFolds } from "./foldState";
import { resetTileImages } from "../../world/paint/paletteImages";
import { TilePalette } from "./TilePalette";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(async () => (vi.isFakeTimers() ? void (await vi.advanceTimersByTimeAsync(1)) : new Promise<void>((resolve) => setTimeout(resolve, 0))));

const tile = (texture: string, role: TileRole, over: Partial<TileJson> = {}): TileJson => ({
  texture,
  texture_index: 0,
  size: [32, 32],
  role,
  kind: 0,
  collision: 0,
  height: "flat",
  origin: "vanilla",
  ...over,
});

/** Five tiles out of order by role: the ground twice, grass, water and a wall; no ramp, no piece. */
const TILESET: TilesetDto = {
  package: 3,
  stored: true,
  tileset: {
    version: 1,
    package: 3,
    tiles: [tile("weg", "ground"), tile("wasser_a", "water"), tile("gras_a", "grass", { kind: 2 }), tile("fels", "wall", { collision: 0x80 }), tile("erde", "ground")],
  },
};

describe("TilePalette", () => {
  let container: HTMLElement;
  let root: Root;
  const onTile = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    onTile.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  type Images = (pkg: number, texture: string) => Promise<ImageBitmap | null>;
  async function show({ tileToken = "0000", depth = "bauen" as Depth, image = (async () => null) as Images, target = undefined as string | undefined } = {}) {
    await act(async () => root.render(<TilePalette tileset={TILESET} tile={tileToken} onTile={onTile} depth={depth} image={image} target={target} />));
    await flush();
  }
  const button = (token: string) => container.querySelector<HTMLButtonElement>(`[data-palette-tile="${token}"]`)!;

  it("tiles_are_grouped_by_role_in_order", async () => {
    await show();
    const palette = container.querySelector("[data-paint-palette]")!;
    expect(palette.querySelector("h3")?.textContent).toBe("Palette");
    const groups = [...palette.querySelectorAll("[data-palette-role]")];
    expect(groups.map((g) => g.getAttribute("data-palette-role"))).toEqual(["ground", "grass", "water", "wall"]);
    // Empty groups (ramp, piece) are left out.
    expect(groups.map((g) => g.querySelector("h4")?.textContent)).toEqual(["Boden", "Gras", "Wasser", "Wand"]);
    expect([...groups[0].querySelectorAll("[data-palette-tile]")].map((b) => b.getAttribute("data-palette-tile"))).toEqual(["0000", "0004"]);
    expect(button("0002").getAttribute("aria-label")).toBe("Kachel 0002, gras_a");
    expect(button("0000").getAttribute("aria-pressed")).toBe("true");
    expect(button("0002").getAttribute("aria-pressed")).toBe("false");
  });

  it("a_tile_without_picture_shows_its_role_colour", async () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ({ drawImage: vi.fn(), clearRect: vi.fn(), imageSmoothingEnabled: true })) as never);
    const picture = {} as ImageBitmap;
    await show({ image: async (_pkg: number, texture: string) => (texture === "weg" ? picture : null) });
    // The ground „weg“ has its picture; grass and water without one show the colour of their role.
    expect(button("0000").querySelector("canvas")).not.toBeNull();
    expect(button("0000").querySelector("[data-palette-color]")).toBeNull();
    expect(button("0002").querySelector("canvas")).toBeNull();
    expect(button("0002").querySelector("[data-palette-color]")?.getAttribute("data-palette-color")).toBe("grass");
    expect(button("0001").querySelector("[data-palette-color]")?.getAttribute("data-palette-color")).toBe("water");
  });

  it("a_reset_of_the_store_asks_for_the_pictures_again", async () => {
    const draw = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ({ drawImage: draw, clearRect: vi.fn(), imageSmoothingEnabled: true })) as never);
    const old = { name: "old" } as unknown as ImageBitmap;
    const fresh = { name: "fresh" } as unknown as ImageBitmap;
    let current = old;
    const image = vi.fn(async (_pkg: number, texture: string) => (texture === "weg" ? current : null));
    await show({ image });
    expect(image).toHaveBeenCalledTimes(5);
    expect(draw).toHaveBeenLastCalledWith(old, 0, 0, 32, 32);
    // Another project (or the package written): the palette asks again and draws the new picture.
    current = fresh;
    await act(async () => resetTileImages());
    await flush();
    expect(image).toHaveBeenCalledTimes(10);
    expect(draw).toHaveBeenLastCalledWith(fresh, 0, 0, 32, 32);
    // Folding the section closed and open draws the current picture, never the old one.
    draw.mockClear();
    const head = container.querySelector<HTMLButtonElement>("[data-paint-palette] h3 button")!;
    act(() => head.click());
    act(() => head.click());
    await flush();
    expect(draw.mock.calls.every(([bitmap]) => bitmap === fresh)).toBe(true);
  });

  it("clicking_chooses_the_tile", async () => {
    await show({ tileToken: "0002r1f" });
    // The chosen tile (turned and mirrored) is pressed and shows both as a small mark.
    expect(button("0002").getAttribute("aria-pressed")).toBe("true");
    expect(button("0002").querySelector("[data-palette-mark]")?.textContent).toBe("90° ⇆");
    expect(button("0000").querySelector("[data-palette-mark]")).toBeNull();
    act(() => button("0004").click());
    expect(onTile).toHaveBeenCalledWith("0004");
  });

  it("while_a_place_is_chosen_a_click_goes_to_that_place", async () => {
    await show();
    expect(container.querySelector("[data-palette-target]")).toBeNull();
    await show({ tileToken: "0000r1", target: "Ecke außen NW" });
    // The palette says where a click goes; the click still names the tile, the owner appends it to the place.
    expect(container.querySelector("[data-palette-target]")?.textContent).toBe("Ein Klick hängt die Kachel (mit Drehung) an den Platz „Ecke außen NW“.");
    act(() => button("0004").click());
    expect(onTile).toHaveBeenCalledExactlyOnceWith("0004");
  });

  it("the_pressed_picture_turns_and_mirrors_like_the_mesh", async () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ({ drawImage: vi.fn(), clearRect: vi.fn(), imageSmoothingEnabled: true })) as never);
    const picture = {} as ImageBitmap;
    await show({ tileToken: "0000r3f", image: async () => picture });
    // The chosen tile's picture: three quarter turns clockwise, mirrored after the turn (CSS by data attribute, no inline style).
    const chosen = button("0000").querySelector("canvas")!;
    expect(chosen.getAttribute("data-palette-turn")).toBe("3");
    expect(chosen.hasAttribute("data-palette-flip")).toBe(true);
    // The other tiles stay plain.
    const other = button("0002").querySelector("canvas")!;
    expect(other.hasAttribute("data-palette-turn")).toBe(false);
    expect(other.hasAttribute("data-palette-flip")).toBe(false);
    await show({ tileToken: "0000r1", image: async () => picture });
    expect(button("0000").querySelector("canvas")!.getAttribute("data-palette-turn")).toBe("1");
    expect(button("0000").querySelector("canvas")!.hasAttribute("data-palette-flip")).toBe(false);
  });

  async function tooltipOf(token: string) {
    await act(async () => {
      button(token).dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      button(token).dispatchEvent(new MouseEvent("mouseenter"));
      button(token).dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
      await vi.advanceTimersByTimeAsync(500);
    });
    return document.querySelector("[role=tooltip]")?.textContent ?? null;
  }

  it("tinkering_shows_kind_and_collision_in_the_tooltip", async () => {
    vi.useFakeTimers();
    await show();
    expect(await tooltipOf("0002")).toBe("Kachel 0002 · gras_a · 32 × 32 Texel · Gras");
    act(() => root.unmount());
    root = createRoot(container);
    await show({ depth: "tuefteln" });
    expect(await tooltipOf("0003")).toBe("Kachel 0003 · fels · 32 × 32 Texel · Wand · kind 0x00, Kollision 0x80");
    // Tüfteln names the number under each tile.
    expect(button("0003").parentElement?.querySelector("[data-palette-number]")?.textContent).toBe("0003");
  });
});
