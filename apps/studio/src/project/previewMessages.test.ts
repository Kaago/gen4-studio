import { afterEach, describe, expect, it, vi } from "vitest";

const openPath = vi.fn<(opts: unknown) => Promise<string | null>>();
vi.mock("../platform/dialogs", () => ({ openPath: (opts: unknown) => openPath(opts) }));

const { choosePreviewFile, previewErrorText } = await import("./previewMessages");

const refused = (message: string) => ({ code: "error", message, asset: null });

describe("previewMessages", () => {
  afterEach(() => openPath.mockReset());

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
});
