// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getTextArchive } from "../../api";
import { archive, flush, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  validateText: vi.fn(async () => []),
  getMatrix: vi.fn(async (index: number) => ({ index, name: "m", width: 1, height: 1, maps: [7] })),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TextList", () => {
  let h: ReturnType<typeof textHarness>;
  const group = (key: string) => h.container.querySelector<HTMLElement>(`[data-text-group="${key}"]`)!;

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset().mockImplementation(async (index) => archive(index, ["Hallo"]));
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => h.unmount());

  it("heads the list with the number of archives and folds a group", async () => {
    h.show(h.base());
    await flush();
    expect(h.container.querySelector("section h2")?.textContent).toBe("Textarchive");
    expect(h.container.querySelector("section header")?.textContent).toContain("435 Archive");
    expect(group("spielweit").getAttribute("aria-expanded")).toBe("true");
    h.click(group("spielweit"));
    expect(group("spielweit").getAttribute("aria-expanded")).toBe("false");
    expect(h.archiveRow(6)).toBeNull();
    expect(h.archiveRow(433)).not.toBeNull();
  });

  it("clears its filter and opens the group when a link opens an archive the list hides", async () => {
    h.show(h.base());
    await flush();
    h.click(group("spielweit"));
    h.type(h.listFilter()!, "0433");
    h.show(h.base({ target: { index: 6, message: null } }));
    await flush();
    expect(h.listFilter()?.value).toBe("");
    expect(h.archiveRow(6)?.getAttribute("aria-selected")).toBe("true");
  });

  it("writes the sum of overlong texts in the singular for one", async () => {
    h.show(h.base());
    await flush();
    act(() => h.problems.replaceText(6, [{ severity: "warning", file: "expanded/textArchives/0006.json", line: 1, col: 1, message: "zu lang", source: "text" }]));
    expect(h.container.textContent).toContain("1 Text zu lang, in 1 geprüften Archiv");
  });
});
