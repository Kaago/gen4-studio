// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { editTextArchive, getTextArchive, validateText } from "../../api";
import { readCss, ruleBody } from "../../ui/cssSource";
import type { Depth } from "../../ui/frame/shellState";
import { archive, flush, header, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  editTextArchive: vi.fn(),
  validateText: vi.fn(),
  getMatrix: vi.fn(async (index: number) => ({ index, name: "m", width: 1, height: 1, maps: [7] })),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TextHand", () => {
  let h: ReturnType<typeof textHarness>;
  const cells = (index: number) => [...h.messageRow(index)!.querySelectorAll('[role="gridcell"]')];

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset().mockResolvedValue(archive(433, ["Hallo {PLAYER}!\\nWie geht es?", "x".repeat(45)]));
    vi.mocked(editTextArchive).mockReset().mockResolvedValue(1);
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => h.unmount());

  it("shows number, text with codes and the longest line of each message, warning above 38", async () => {
    await h.open433();
    expect(cells(0).map((c) => c.textContent)).toEqual(["000", "Hallo {PLAYER}!⏎Wie geht es?", "12 von 38"]);
    expect(cells(0)[2].querySelector("[data-over]")).toBeNull();
    expect(cells(1)[2].querySelector("[data-over]")?.textContent).toBe("45 von 38");
    // The column of the selected message: its title, the field and where the archive is used.
    expect(h.container.querySelector('[aria-label="Nachricht bearbeiten"] h2')?.textContent).toBe("Nachricht 000");
    expect(h.container.textContent).toContain("Verwendet in");
  });

  /** Opens archive 433, used by one header without a location name, or archive 434, used by none. */
  const usedIn = async (index: number, depth: Depth) => {
    h.show(h.base({ headers: [header(12, 433)], depth }));
    await flush();
    h.click(h.archiveRow(index));
    await flush();
    return h.container.querySelector<HTMLElement>('[aria-label="Nachricht bearbeiten"]')?.textContent ?? "";
  };

  it("says_no_raster_matrix_or_header_in_building", async () => {
    expect(await usedIn(433, "bauen")).toContain("Verwendet inOrt0012");
    expect(await usedIn(434, "bauen")).toContain("Verwendet inkeinem Ort");
  });

  it("keeps_the_technical_words_in_tinkering", async () => {
    expect(await usedIn(433, "tuefteln")).toContain("Verwendet inHeader0012");
    expect(await usedIn(434, "tuefteln")).toContain("Verwendet inkeinem Header");
  });

  it("keeps the removal in reach when an archive is used by hundreds of headers", () => {
    // Archive 0023 on Platin is used by 190 headers: the list scrolls on its own instead of pushing the action 5000 px down.
    const css = readCss("./TextHand.module.css", import.meta.url);
    expect(ruleBody(css, ".usedIn")).toMatch(/max-height:\s*calc\(var\(--g4-raum-/);
    expect(ruleBody(css, ".usedIn")).toMatch(/overflow-y:\s*auto/);
  });

  it("starts the table of another archive at its top", async () => {
    vi.mocked(getTextArchive).mockImplementation(async (index) => archive(index, Array.from({ length: 300 }, (_, i) => `Text ${index}.${i}`)));
    await h.open433();
    const body = () => h.container.querySelector<HTMLElement>("[data-text-table-body]")!;
    act(() => {
      body().scrollTop = 200 * 28;
      body().dispatchEvent(new Event("scroll"));
    });
    expect(h.messageRow(0)).toBeNull();
    h.click(h.archiveRow(5));
    await flush();
    expect(body().scrollTop).toBe(0);
    expect(h.messageRow(0)?.textContent).toContain("Text 5.0");
  });

  it("offers to create a message for an archive without one", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, [])).mockResolvedValue(archive(433, [""]));
    await h.open433();
    expect(h.container.textContent).toContain("Dieses Archiv hat keine Nachricht.");
    const create = [...h.container.querySelectorAll("button")].filter((b) => b.textContent === "Nachricht anlegen");
    expect(create.length).toBeGreaterThan(0);
    h.click(create[create.length - 1]);
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, "Append");
    expect(h.messageRow(0)).not.toBeNull();
  });
});
