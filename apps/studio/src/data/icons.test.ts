import { beforeEach, describe, expect, it, vi } from "vitest";
import { getItemIcon, getMonIcon } from "../api";
import { itemIcon, itemIconKey, monIcon, peekIcon, resetIcons } from "./icons";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  getItemIcon: vi.fn(),
  getMonIcon: vi.fn(),
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
});
