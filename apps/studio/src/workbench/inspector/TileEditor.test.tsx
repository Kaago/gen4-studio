// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tilesetEdit, tilesetSave } from "../../api";
import type { TileBehavior } from "../../api";
import type { TileJson, TilesetDto } from "../../api.world";
import type { Depth } from "../../ui/frame/shellState";
import { openSelect, typeInto } from "../../ui/primitives/testUtils";
import { resetSectionFolds } from "./foldState";
import { TileEditor } from "./TileEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), tilesetEdit: vi.fn(), tilesetSave: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const tile = (over: Partial<TileJson> = {}): TileJson => ({
  texture: "weg",
  texture_index: 0,
  size: [32, 32],
  role: "ground",
  kind: 0,
  collision: 0,
  height: "flat",
  origin: "vanilla",
  ...over,
});

/** 42 tiles; tile 41 (hex 0029) is the edited one: kind 0x02, collision 0x05 (the low bits stay when blocking). */
const tileset = (stored = true, over: Partial<TileJson> = {}): TilesetDto => {
  const tiles = Array.from({ length: 42 }, () => tile());
  tiles[41] = tile({ texture: "gras_a", kind: 2, collision: 0x05, ...over });
  return { package: 6, stored, tileset: { version: 1, package: 6, tiles } };
};

const NAMES = new Map<number, TileBehavior>([
  [0, { value: 0, name: "Normal", class: "normal" }],
  [2, { value: 2, name: "Hohes Gras", class: "grass" }],
]);

describe("TileEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onChanged = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    vi.mocked(tilesetEdit).mockReset().mockResolvedValue({ package: 6, tiles: 1, change: 1 });
    vi.mocked(tilesetSave).mockReset().mockResolvedValue({ package: 6, file: "data/tilesets/0006.json", tiles: 42, vanilla: 40, unclear: 2, change: 1 });
    onChanged.mockReset();
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show({ set = tileset(), family = "pt" as "pt" | "hgss", depth = "bauen" as Depth } = {}) {
    await act(async () =>
      root.render(<TileEditor tileset={set} tile={41} family={family} depth={depth} onChanged={onChanged} onError={onError} names={async () => NAMES} />),
    );
    await flush();
  }
  const editor = () => container.querySelector<HTMLElement>("[data-tile-editor]")!;
  const select = (label: string) => container.querySelector<HTMLElement>(`[role=combobox][aria-label='${label}']`)!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`);
  const checkbox = () => container.querySelector<HTMLElement>("[role=checkbox]")!;
  const typeAndBlur = async (label: string, text: string) => {
    await act(async () => void input(label)!.focus());
    typeInto(input(label)!, text);
    await act(async () => void input(label)!.blur());
    await flush();
  };

  it("each_confirmed_field_is_one_tileset_edit", async () => {
    await show();
    expect(editor().querySelector("h3")?.textContent).toBe("Kachel 0029");
    expect(editor().textContent).toContain("Aus den Vanilla-Karten abgeleitet");
    // Rolle.
    openSelect(select("Rolle"));
    expect(options().map((o) => o.textContent)).toEqual(["Boden", "Wand", "Rampe", "Wasser", "Gras", "Teilmodell"]);
    await act(async () => void options().find((o) => o.textContent === "Gras")!.click());
    await flush();
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(6, [{ tile: 41, role: "grass" }]);
    expect(onChanged).toHaveBeenCalledTimes(1);
    // Blockiert: bit 7 set, the other bits of the collision byte stay.
    await act(async () => void checkbox().click());
    await flush();
    expect(tilesetEdit).toHaveBeenLastCalledWith(6, [{ tile: 41, collision: 0x85 }]);
    // Gelände by its name (Bauen).
    openSelect(select("Gelände"));
    await act(async () => void options().find((o) => o.textContent === "Normal")!.click());
    await flush();
    expect(tilesetEdit).toHaveBeenLastCalledWith(6, [{ tile: 41, kind: 0 }]);
    // Wand: a tile number as the palette writes it, empty clears it.
    await typeAndBlur("Wand", "0003");
    expect(tilesetEdit).toHaveBeenLastCalledWith(6, [{ tile: 41, wall: 3 }]);
    expect(tilesetEdit).toHaveBeenCalledTimes(4);
    expect(onChanged).toHaveBeenCalledTimes(4);
    // A refusal goes to onError, nothing else changes.
    vi.mocked(tilesetEdit).mockRejectedValueOnce(new Error("nein"));
    await act(async () => void checkbox().click());
    await flush();
    expect(onError).toHaveBeenCalledTimes(1);
    // Cleared wall and a blocked tile unblocked.
    await show({ set: tileset(true, { wall: 3, collision: 0x85, origin: "user" }) });
    expect(editor().textContent).toContain("Von dir gesetzt");
    expect(input("Wand")!.value).toBe("0003");
    await typeAndBlur("Wand", "");
    expect(tilesetEdit).toHaveBeenLastCalledWith(6, [{ tile: 41, wall: null }]);
    await act(async () => void checkbox().click());
    await flush();
    expect(tilesetEdit).toHaveBeenLastCalledWith(6, [{ tile: 41, collision: 0x05 }]);
  });

  it("sound_shows_only_for_hgss", async () => {
    await show();
    expect(input("Schrittgeräusch")).toBeNull();
    await show({ family: "hgss", set: tileset(true, { sound: 12 }) });
    expect(input("Schrittgeräusch")!.value).toBe("12");
    await typeAndBlur("Schrittgeräusch", "128");
    expect(tilesetEdit).not.toHaveBeenCalled();
    await typeAndBlur("Schrittgeräusch", "127");
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(6, [{ tile: 41, sound: 127 }]);
  });

  it("without_file_the_fields_wait_for_tileset_save", async () => {
    await show({ set: tileset(false, { origin: "unclear" }) });
    expect(editor().textContent).toContain("Nicht eindeutig, Vorgabe Boden");
    const save = container.querySelector<HTMLButtonElement>("[data-tileset-save]")!;
    expect(save.textContent).toBe("Tileset im Projekt anlegen");
    expect(editor().textContent).toContain("Die Vorgaben stehen dann in data/tilesets/0006.json und gelten für alle Welt-Karten dieses Pakets.");
    expect(input("Wand")!.disabled).toBe(true);
    expect(select("Rolle").hasAttribute("data-disabled")).toBe(true);
    expect(checkbox().hasAttribute("data-disabled")).toBe(true);
    await act(async () => void save.click());
    await flush();
    expect(tilesetSave).toHaveBeenCalledExactlyOnceWith(6);
    expect(onChanged).toHaveBeenCalledTimes(1);
    expect(tilesetEdit).not.toHaveBeenCalled();
    // With the file the button is gone and the fields open.
    await show();
    expect(container.querySelector("[data-tileset-save]")).toBeNull();
    expect(input("Wand")!.disabled).toBe(false);
  });

  it("tinkering_shows_kind_in_hex", async () => {
    await show();
    // Bauen: the behaviour by its name.
    expect(select("Gelände").textContent).toContain("Hohes Gras");
    await show({ depth: "tuefteln" });
    expect(select("Gelände")).toBeNull();
    expect(input("Gelände")!.value).toBe("0x2");
    expect(editor().textContent).toContain("0x02 Hohes Gras");
    await typeAndBlur("Gelände", "0x10");
    expect(tilesetEdit).toHaveBeenCalledExactlyOnceWith(6, [{ tile: 41, kind: 0x10 }]);
  });
});
