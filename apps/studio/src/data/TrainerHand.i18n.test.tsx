// @vitest-environment jsdom
import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetIcons } from "./icons";
import { flush } from "./dataTesting";
import { trainerHarness, wireApi } from "./trainer/trainerTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  getTrainerSprite: vi.fn(),
  getBattleSprite: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  getEvents: vi.fn(),
  createTrainer: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("the trainer editor in English", () => {
  let h: ReturnType<typeof trainerHarness>;

  beforeEach(async () => {
    await i18next.changeLanguage("en");
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = trainerHarness();
  });
  afterEach(async () => {
    h.unmount();
    await i18next.changeLanguage("de");
  });

  it("names its tabs Party and Trainer", async () => {
    await h.open(12, "bauen");
    await flush();
    const tabs = h.tabs().map((t) => t.textContent ?? "");
    expect(tabs.some((t) => t.startsWith("Party"))).toBe(true);
    expect(tabs).toContain("Trainer");
    expect(tabs.some((t) => t.startsWith("Team"))).toBe(false);
  });
});
