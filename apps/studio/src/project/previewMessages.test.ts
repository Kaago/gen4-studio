import { afterEach, describe, expect, it, vi } from "vitest";

const openPath = vi.fn<(opts: unknown) => Promise<string | null>>();
vi.mock("../platform/dialogs", () => ({ openPath: (opts: unknown) => openPath(opts) }));
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), projectPreviewSet: vi.fn() }));

const { projectPreviewSet } = await import("../api");
const { listToasts, resetToasts } = await import("../ui/frame/toasts");
const { changePreview, choosePreviewFile, previewErrorText } = await import("./previewMessages");

const refused = (message: string) => ({ code: "error", message, asset: null });

describe("previewMessages", () => {
  afterEach(() => {
    openPath.mockReset();
    vi.mocked(projectPreviewSet).mockReset();
    resetToasts();
  });
  const toasts = () => listToasts().map((t) => [t.kind, t.text]);
  const own = { source: "eigen" as const, png: "T1dO", standardMap: null };

  it("says the spec's sentence for each of the three codes", () => {
    expect(previewErrorText(refused("preview.format: neither PNG nor JPEG"))).toBe("Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.");
    expect(previewErrorText(refused("preview.too_large: 9000 × 9000 pixels"))).toBe(
      "Das Bild ist zu groß (höchstens 32 MB und 8192 × 8192 Pixel). Wähle ein kleineres Bild.",
    );
    expect(previewErrorText(refused("preview.unreadable: /x.png: No such file"))).toBe("Das Bild lässt sich nicht lesen. Wähle eine andere Datei.");
  });

  it("any other error keeps its own text", () => {
    expect(previewErrorText(refused("no project open"))).toBe("no project open");
    expect(previewErrorText("bridge down")).toBe("bridge down");
  });

  it("chooses a PNG or JPEG file in the dialog Vorschaubild wählen", async () => {
    openPath.mockResolvedValue("/Users/du/Bild.png");
    await expect(choosePreviewFile()).resolves.toBe("/Users/du/Bild.png");
    expect(openPath).toHaveBeenCalledWith({
      directory: false,
      title: "Vorschaubild wählen",
      filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }],
    });
    openPath.mockResolvedValue(null);
    await expect(choosePreviewFile()).resolves.toBeNull();
  });

  it("change_preview_chooses_a_file_writes_it_and_says_so", async () => {
    openPath.mockResolvedValue("/bilder/stadt.jpg");
    vi.mocked(projectPreviewSet).mockResolvedValue(own);
    // With a path: that project's picture (Start); without one: the open project's (Technik).
    await expect(changePreview("choose", "/p")).resolves.toEqual(own);
    expect(projectPreviewSet).toHaveBeenLastCalledWith({ path: "/p", file: "/bilder/stadt.jpg" });
    await expect(changePreview("choose")).resolves.toEqual(own);
    expect(projectPreviewSet).toHaveBeenLastCalledWith({ file: "/bilder/stadt.jpg" });
    expect(toasts()).toEqual([
      ["info", "Vorschaubild gesetzt"],
      ["info", "Vorschaubild gesetzt"],
    ]);
  });

  it("change_preview_removes_without_a_dialog", async () => {
    const none = { source: null, png: null, standardMap: null };
    vi.mocked(projectPreviewSet).mockResolvedValue(none);
    await expect(changePreview("remove", "/p")).resolves.toEqual(none);
    expect(openPath).not.toHaveBeenCalled();
    expect(projectPreviewSet).toHaveBeenLastCalledWith({ path: "/p" });
    await changePreview("remove");
    expect(projectPreviewSet).toHaveBeenLastCalledWith({});
    expect(toasts()).toEqual([
      ["info", "Vorschaubild entfernt"],
      ["info", "Vorschaubild entfernt"],
    ]);
  });

  it("change_preview_cancelled_writes_nothing_and_says_nothing", async () => {
    openPath.mockResolvedValue(null);
    await expect(changePreview("choose", "/p")).resolves.toBeNull();
    expect(projectPreviewSet).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });

  it("change_preview_says_a_refusal_with_its_sentence", async () => {
    openPath.mockResolvedValue("/notiz.txt");
    vi.mocked(projectPreviewSet).mockRejectedValue(refused("preview.format: neither PNG nor JPEG"));
    await expect(changePreview("choose", "/p")).resolves.toBeNull();
    expect(toasts()).toEqual([["error", "Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei."]]);
  });

  it("change_preview_says_a_failing_dialog_instead_of_rejecting", async () => {
    openPath.mockRejectedValue(refused("dialog failed"));
    await expect(changePreview("choose", "/p")).resolves.toBeNull();
    expect(projectPreviewSet).not.toHaveBeenCalled();
    expect(toasts()).toEqual([["error", "dialog failed"]]);
  });
});
