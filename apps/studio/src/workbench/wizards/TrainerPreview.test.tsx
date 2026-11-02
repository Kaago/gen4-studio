// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getBattleSprite, getMonIcon, getTrainerSprite } from "../../api";
import type { NameTablesDto } from "../../api";
import { resetIcons } from "../../data/icons";
import { TrainerPreview } from "./TrainerPreview";
import type { TeamRow } from "./TrainerPreview";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getMonIcon: vi.fn(),
  getBattleSprite: vi.fn(),
  getTrainerSprite: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const names: NameTablesDto = {
  species: ["-----", "Bisasam", "Bisaknosp", "Bisaflor", "Glumanda"],
  moves: [],
  items: [],
  abilities: [],
  types: [],
  classes: ["Pokémon-Trainer", "Wanderer", "Käfersammler"],
  forms: [],
  archives: { species: null, moves: null, items: null, abilities: null, types: null, classes: null, forms: null },
  sounds: [],
};

describe("TrainerPreview", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    resetIcons();
    // Every picture gets a URL of its own, named after what was asked for.
    vi.mocked(getTrainerSprite).mockReset().mockImplementation(async (cls) => new TextEncoder().encode(`trainer:${cls}`).buffer as ArrayBuffer);
    vi.mocked(getMonIcon).mockReset().mockImplementation(async (species) => new TextEncoder().encode(`mon:${species}`).buffer as ArrayBuffer);
    vi.mocked(getBattleSprite).mockReset().mockImplementation(async ({ species }) => new TextEncoder().encode(`battle:${species}`).buffer as ArrayBuffer);
    URL.createObjectURL = vi.fn((blob: Blob) => `blob:${(blob as Blob & { size: number }).size}`);
    URL.revokeObjectURL = vi.fn();
    onSlot.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
  const onSlot = vi.fn<(slot: number) => void>();
  async function show(cls: number, name: string, party: TeamRow[]) {
    await act(async () => root.render(<TrainerPreview cls={cls} name={name} party={party} names={names} classes={names.classes} onSlot={onSlot} />));
    await flush();
  }
  const preview = () => container.querySelector<HTMLElement>("[data-trainer-preview]")!;
  const foe = () => preview().querySelector('[data-ds-status="foe"]')!;
  const segments = () => [...preview().querySelectorAll<HTMLButtonElement>("[data-foe] button")];
  const choose = (label: string) => act(async () => void segments().find((b) => b.textContent === label)?.dispatchEvent(new MouseEvent("click", { bubbles: true })));

  it("shows the class sprite and the first mon", async () => {
    await show(1, "Jens", [
      { species: 1, level: 5 },
      { species: 4, level: 12 },
    ]);
    expect(preview()).not.toBeNull();
    // The class picture with the class name beside it.
    expect(getTrainerSprite).toHaveBeenCalledWith(1);
    expect(preview().querySelector("[data-trainer-sprite] img")).not.toBeNull();
    expect(preview().textContent).toContain("Wanderer");
    expect(preview().textContent).toContain("Jens");
    // The battle starts on the trainer: class and name in the box, no level, no own side.
    expect(preview().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
    expect(foe().textContent).toContain("Wanderer Jens");
    expect(foe().textContent).not.toContain("Lv.");
    expect(preview().querySelector('[data-ds-status="own"]')).toBeNull();
    // The team as cards with their menu icons.
    const cards = [...preview().querySelectorAll<HTMLElement>("[data-preview-slot]")];
    expect(cards.map((c) => c.getAttribute("data-preview-slot"))).toEqual(["0", "1"]);
    expect(cards.map((c) => c.getAttribute("data-species"))).toEqual(["1", "4"]);
    expect(cards[0]?.textContent).toContain("Bisasam");
    expect(cards[1]?.textContent).toContain("Lv. 12");
    expect(getMonIcon).toHaveBeenCalledWith(1);
    expect(getMonIcon).toHaveBeenCalledWith(4);
    expect(cards.every((c) => c.querySelector("img[data-sprite=picture]") !== null)).toBe(true);
    // No battle picture is asked for before the first Pokémon is shown.
    expect(getBattleSprite).not.toHaveBeenCalled();
    // A card asks the wizard to pick the species of its place.
    await act(async () => void cards[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(onSlot).toHaveBeenCalledExactlyOnceWith(1);
  });

  it("switches the foe side", async () => {
    await show(1, "Jens", [{ species: 4, level: 12 }]);
    expect(segments().map((b) => b.textContent)).toEqual(["Trainer", "Erstes Pokémon"]);
    await choose("Erstes Pokémon");
    await flush();
    expect(preview().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("pokemon");
    expect(foe().textContent).toContain("Glumanda");
    expect(foe().textContent).toContain("Lv. 12");
    expect(getBattleSprite).toHaveBeenCalledExactlyOnceWith({ species: 4, side: "front" });
    expect(preview().querySelector('[data-ds-status="own"]')).toBeNull();
    // The species changes in the form: the battle follows.
    await show(1, "Jens", [{ species: 2, level: 30 }]);
    expect(foe().textContent).toContain("Bisaknosp");
    expect(foe().textContent).toContain("Lv. 30");
    expect(getBattleSprite).toHaveBeenLastCalledWith({ species: 2, side: "front" });
    await choose("Trainer");
    expect(preview().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
    expect(foe().textContent).toContain("Wanderer Jens");
  });

  it("without a species the battle side is the trainer only", async () => {
    await show(2, "", [{ species: null, level: 5 }]);
    expect(segments().map((b) => b.textContent)).toEqual(["Trainer"]);
    expect(preview().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
    // Without a name the class alone stands in the box.
    expect(foe().textContent?.trim()).toBe("Käfersammler");
    // The free place is a dashed card that says so; no icon is asked for.
    const card = preview().querySelector<HTMLElement>('[data-preview-slot="0"]')!;
    expect(card.getAttribute("data-state")).toBe("empty");
    expect(card.textContent).toContain("Platz 1 leer");
    expect(getMonIcon).not.toHaveBeenCalled();
    expect(getBattleSprite).not.toHaveBeenCalled();
    // The first place decides: a species in the second place only does not make a first Pokémon.
    await show(2, "Uwe", [
      { species: null, level: 5 },
      { species: 3, level: 9 },
    ]);
    expect(segments().map((b) => b.textContent)).toEqual(["Trainer"]);
    expect(foe().textContent).toContain("Käfersammler Uwe");
  });
});
