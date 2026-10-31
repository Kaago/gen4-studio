import { beforeEach, describe, expect, it, vi } from "vitest";
import { getBattleSprite, getItemIcon, getMonIcon, getTrainerSprite } from "../api";
import { battleSprite, battleSpriteKey, itemIcon, itemIconKey, monIcon, peekIcon, resetIcons, trainerSprite, trainerSpriteKey } from "./icons";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  getItemIcon: vi.fn(),
  getMonIcon: vi.fn(),
  getBattleSprite: vi.fn(),
  getTrainerSprite: vi.fn(),
}));

const png = () => new Uint8Array([137, 80, 78, 71]).buffer;

describe("icons", () => {
  let made = 0;
  beforeEach(() => {
    resetIcons();
    made = 0;
    URL.createObjectURL = vi.fn(() => `blob:${++made}`);
    URL.revokeObjectURL = vi.fn();
    vi.mocked(getItemIcon).mockReset().mockImplementation(async () => png());
    vi.mocked(getMonIcon).mockReset().mockImplementation(async () => png());
    vi.mocked(getBattleSprite).mockReset().mockImplementation(async () => png());
    vi.mocked(getTrainerSprite).mockReset().mockImplementation(async () => png());
  });

  it("asks_once_per_icon_and_palette_and_shares_the_url", async () => {
    const potion = { data: 17, icon: 30, palette: 31, gba: 0 };
    expect(itemIconKey(potion)).toBe("item:30:31");
    const [a, b] = await Promise.all([itemIcon(17, potion), itemIcon(18, { ...potion, data: 18 })]);
    expect(a).toBe("blob:1");
    expect(b).toBe("blob:1");
    expect(getItemIcon).toHaveBeenCalledOnce();
    expect(peekIcon("item:30:31")).toBe("blob:1");
  });

  it("remembers_a_missing_icon_and_a_failed_request_until_another_project", async () => {
    vi.mocked(getMonIcon).mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("weg"));
    expect(await monIcon(1)).toBeNull();
    expect(await monIcon(1)).toBeNull();
    expect(getMonIcon).toHaveBeenCalledOnce();
    // A failed icon is not asked for again while the project stays open (a row scrolled back into view).
    expect(await monIcon(2)).toBeNull();
    expect(peekIcon("mon:2")).toBeNull();
    expect(await monIcon(2)).toBeNull();
    expect(getMonIcon).toHaveBeenCalledTimes(2);
    // Another project asks again.
    resetIcons();
    expect(await monIcon(2)).toBe("blob:1");
    expect(getMonIcon).toHaveBeenCalledTimes(3);
  });

  it("frees_the_urls_and_forgets_late_answers_on_reset", async () => {
    await monIcon(397);
    let answer: (bytes: ArrayBuffer) => void = () => {};
    vi.mocked(getMonIcon).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
    const late = monIcon(398);
    resetIcons();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:1");
    expect(peekIcon("mon:397")).toBeUndefined();
    answer(png());
    expect(await late).toBeNull();
    expect(peekIcon("mon:398")).toBeUndefined();
  });

  it("keys_battle_sprites_by_every_part_of_the_request_with_the_defaults_filled_in", () => {
    expect(battleSpriteKey({ species: 25, side: "front" })).toBe("battle:25:0:front:0:0:0");
    expect(battleSpriteKey({ species: 25, form: 0, side: "front", female: false, shiny: false, frame: 0 })).toBe("battle:25:0:front:0:0:0");
    expect(battleSpriteKey({ species: 479, form: 2, side: "back", female: true, shiny: true, frame: 1 })).toBe("battle:479:2:back:1:1:1");
    expect(trainerSpriteKey(7)).toBe("trainer:7");
  });

  it("loads_each_battle_and_trainer_sprite_once_per_key", async () => {
    const [a, b] = await Promise.all([battleSprite({ species: 25, side: "front" }), battleSprite({ species: 25, form: 0, side: "front" })]);
    expect(a).toBe("blob:1");
    expect(b).toBe("blob:1");
    expect(getBattleSprite).toHaveBeenCalledOnce();
    expect(getBattleSprite).toHaveBeenCalledWith({ species: 25, side: "front" });
    // Another side is another image.
    expect(await battleSprite({ species: 25, side: "back", shiny: true })).toBe("blob:2");
    expect(getBattleSprite).toHaveBeenCalledTimes(2);
    expect(peekIcon("battle:25:0:back:0:1:0")).toBe("blob:2");

    vi.mocked(getTrainerSprite).mockResolvedValueOnce(null);
    expect(await trainerSprite(3)).toBeNull();
    expect(await trainerSprite(3)).toBeNull();
    expect(getTrainerSprite).toHaveBeenCalledOnce();
    expect(getTrainerSprite).toHaveBeenCalledWith(3);
    expect(peekIcon("trainer:3")).toBeNull();
    // Another project asks again.
    resetIcons();
    expect(await trainerSprite(3)).toBe("blob:3");
    expect(getTrainerSprite).toHaveBeenCalledTimes(2);
  });
});
