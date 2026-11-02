// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogsOfHeader, getTextArchive } from "../../api";
import { figure, flush, header, headerDialogs, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  dialogsOfHeader: vi.fn(),
  dialogOf: vi.fn(),
  validateText: vi.fn(async () => []),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DialogList", () => {
  let h: ReturnType<typeof textHarness>;
  const place = (name: string) => h.container.querySelector<HTMLElement>(`[data-dialog-place="${name}"]`);
  const headerRow = (id: number) => h.container.querySelector<HTMLElement>(`[data-dialog-header="${id}"]`);

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset();
    vi.mocked(dialogsOfHeader)
      .mockReset()
      .mockImplementation(async (id) =>
        id === 12
          ? headerDialogs(12, [figure("object", 0), figure("object", 1, { source: { kind: "none" }, messages: 0, pages: 0, first_text: null }), figure("bg", 0)])
          : id === 13
            ? headerDialogs(13, [figure("object", 0)])
            : headerDialogs(id, [figure("object", 0, { source: { kind: "none" }, messages: 0, pages: 0, first_text: null })]),
      );
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => h.unmount());

  it("lists the places in the order of the world, folded, and reads no dialog until one opens", async () => {
    h.show(h.base({ depth: "erzaehlen", headers: [header(14, 5, "Sandgemme"), header(12, 433, "Zweiblattdorf"), header(13, 433, "Zweiblattdorf"), header(20, 7, "")] }));
    await flush();
    const list = h.container.querySelector("[data-dialog-list]");
    expect(list).not.toBeNull();
    // buildPlaces: by the first header id, the nameless place last.
    expect([...list!.querySelectorAll("[data-dialog-place]")].map((e) => e.getAttribute("data-dialog-place"))).toEqual(["Zweiblattdorf", "Sandgemme", "Ohne Ortsnamen"]);
    expect(h.container.querySelector("[data-dialog-header]")).toBeNull();
    expect(h.container.querySelector("[data-archive]")).toBeNull();
    // No archive numbers in Erzählen.
    expect(list!.textContent).not.toContain("0433");
    expect(dialogsOfHeader).not.toHaveBeenCalled();
    expect(getTextArchive).not.toHaveBeenCalled();
  });

  it("opens a place: its headers say how many figures speak there", async () => {
    h.show(h.base({ depth: "erzaehlen" }));
    await flush();
    h.click(place("Zweiblattdorf"));
    await flush();
    expect(place("Zweiblattdorf")?.getAttribute("aria-expanded")).toBe("true");
    expect(dialogsOfHeader).toHaveBeenCalledTimes(2);
    expect(dialogsOfHeader).toHaveBeenCalledWith(12);
    expect(dialogsOfHeader).toHaveBeenCalledWith(13);
    expect(headerRow(12)?.textContent).toContain("2 Figuren sprechen");
    expect(headerRow(13)?.textContent).toContain("1 Figur spricht");
    expect(headerRow(14)).toBeNull();
    h.click(place("Sandgemme"));
    await flush();
    expect(headerRow(14)?.textContent).toContain("Keine Figur spricht");
    // Folded and opened again: the dialogs are not read again.
    h.click(place("Zweiblattdorf"));
    h.click(place("Zweiblattdorf"));
    await flush();
    expect(dialogsOfHeader).toHaveBeenCalledTimes(3);
  });

  it("a header without figures and one that cannot be read say so", async () => {
    vi.mocked(dialogsOfHeader).mockImplementation(async (id) => {
      if (id === 13) throw { code: "error", message: "Eventdatei gesperrt", asset: null };
      return headerDialogs(id, []);
    });
    h.show(h.base({ depth: "erzaehlen" }));
    await flush();
    h.click(place("Zweiblattdorf"));
    await flush();
    expect(headerRow(12)?.textContent).toContain("Keine Figuren");
    expect(headerRow(13)?.textContent).toContain("Nicht lesbar");
  });

  it("a click on a header selects it, the filter narrows the places", async () => {
    h.show(h.base({ depth: "erzaehlen" }));
    await flush();
    h.click(place("Zweiblattdorf"));
    await flush();
    h.click(headerRow(13));
    await flush();
    expect(headerRow(13)?.getAttribute("aria-selected")).toBe("true");
    expect(headerRow(12)?.getAttribute("aria-selected")).toBe("false");
    h.type(h.container.querySelector<HTMLInputElement>('input[aria-label="Orte filtern"]')!, "sand");
    expect(place("Zweiblattdorf")).toBeNull();
    expect(place("Sandgemme")).not.toBeNull();
  });

  it("a filter shows the matching headers without reading their figures", async () => {
    h.show(h.base({ depth: "erzaehlen" }));
    await flush();
    h.type(h.container.querySelector<HTMLInputElement>('input[aria-label="Orte filtern"]')!, "e");
    await flush();
    // Both places match and show their headers, but nothing asks the session.
    expect(headerRow(12)).not.toBeNull();
    expect(headerRow(14)).not.toBeNull();
    expect(headerRow(12)?.textContent).not.toMatch(/Figur|Lese/);
    expect(dialogsOfHeader).not.toHaveBeenCalled();
    // A place the user opens counts its figures.
    h.click(place("Sandgemme"));
    await flush();
    expect(dialogsOfHeader).toHaveBeenCalledTimes(1);
    expect(dialogsOfHeader).toHaveBeenCalledWith(14);
    expect(headerRow(14)?.textContent).toContain("Keine Figur spricht");
  });
});
