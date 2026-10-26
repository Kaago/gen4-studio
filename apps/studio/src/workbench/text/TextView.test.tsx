// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getTextArchive, validateText } from "../../api";
import { archive, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  validateText: vi.fn(),
  getMatrix: vi.fn(async (index: number) => ({ index, name: "m", width: 1, height: 1, maps: [7] })),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => "data:image/png;base64,AA==", request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TextView", () => {
  let h: ReturnType<typeof textHarness>;
  const view = () => h.container.querySelector<HTMLElement>('[aria-label="Vorschau"]')!;
  const cards = () => [...view().querySelectorAll<HTMLButtonElement>("[data-page]")];

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset().mockResolvedValue(archive(433, ["Eins\\nZwei\\rDrei, viel länger als die anderen Zeilen hier\\fVier", "Kurz"]));
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => h.unmount());

  it("heads the view with the message, its archive and place, and the number of pages", async () => {
    await h.open433();
    expect(view().querySelector("h2")?.textContent).toBe("Nachricht 000");
    expect(view().textContent).toContain("Archiv 0433, Zweiblattdorf");
    expect(view().textContent).toContain("3 Seiten");
    expect(view().textContent).toContain("Seite 1 von 3, 256 × 192, 2-fach");
    // The picture of the first header's map lies behind the box.
    expect(view().querySelector("[data-map-picture]")).not.toBeNull();
  });

  it("has_no_head_title_that_repeats_the_tab", async () => {
    await h.open433();
    // The dock tab says "Vorschau"; the head names the message shown.
    expect([...view().querySelectorAll("h2")].map((t) => t.textContent)).toEqual(["Nachricht 000"]);
    expect([...h.container.querySelectorAll("h2")].map((t) => t.textContent)).not.toContain("Vorschau");
  });

  it("lists the pages as cards with the characters of each line, and a card click shows its page", async () => {
    await h.open433();
    expect(cards().map((c) => c.querySelector("[data-page-title]")?.textContent)).toEqual(["Seite 1", "Seite 2", "Seite 3"]);
    expect(cards()[0].getAttribute("aria-pressed")).toBe("true");
    expect([...cards()[0].querySelectorAll("[data-line-width]")].map((e) => e.textContent)).toEqual(["4 von 38", "4 von 38"]);
    expect(cards()[1].querySelector("[data-line-width][data-over]")?.textContent).toBe("45 von 38");
    h.click(cards()[2]);
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 3 von 3");
    expect(cards()[2].getAttribute("aria-pressed")).toBe("true");
    expect(h.button("Weiterblättern")?.disabled).toBe(true);
  });

  it("pages with the arrow keys while the focus is in the view, and back with Zurück", async () => {
    await h.open433();
    h.key(cards()[0], { key: "ArrowRight" });
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 2 von 3");
    h.key(view(), { key: "ArrowRight" });
    h.key(view(), { key: "ArrowRight" });
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 3 von 3");
    h.click(h.button("Zurück"));
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 2 von 3");
    h.key(view(), { key: "ArrowLeft" });
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 1 von 3");
    expect(h.button("Zurück")?.disabled).toBe(true);
  });

  it("follows what is typed in the field before it is committed, and starts at the first page for another message", async () => {
    await h.open433();
    h.click(h.button("Weiterblättern"));
    h.type(h.textarea()!, "Neu\\rSeite");
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 2 von 2");
    expect(h.screen()?.textContent).toContain("Seite");
    h.click(h.messageRow(1));
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 1 von 1");
    expect(view().querySelector("h2")?.textContent).toBe("Nachricht 001");
  });
});
