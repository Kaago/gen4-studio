import i18next from "i18next";
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

  it("any other error says in German what to check, the bridge's German refusals keep their text", () => {
    const sentence = "Das Vorschaubild ließ sich nicht ändern. Prüfe, ob der Projektordner noch da ist und du darin schreiben darfst.";
    expect(previewErrorText(refused("/Users/du/Spiele/pt: project folder not found"))).toBe(sentence);
    expect(previewErrorText(refused("Permission denied (os error 13)"))).toBe(sentence);
    expect(previewErrorText("bridge down")).toBe(sentence);
    for (const own of [
      "Pfad liegt außerhalb der erlaubten Ordner: /tmp/x.png",
      "Pfad ist kein gültiger Text: /x",
      "Ungültige Argumente für project_preview_set: y",
      "Pfad nicht lesbar: /x.png: No such file",
      "Ordner nicht lesbar: /x: Permission denied",
      "Keine Datei: /x",
      "Kein Ordner: /x",
      "Kein Dateiname: /",
      "Originale (_DSPRE_contents) sind gesperrt: /p_DSPRE_contents/x.png",
      "Originale sind gesperrt, nur Kopien unter /k: /o/x.png",
      "Spielstände (_saves) sind gesperrt: /p/_saves/x.png",
      "Nur das offene Projekt: /q/x.png",
      "Brücke nicht erreichbar",
      "Kein Tauri und keine Brücke",
    ]) {
      expect(previewErrorText(refused(own))).toBe(own);
    }
    // The app's own failures (an Error it throws: no map at the chosen cell, a lost WebGL context) say their German sentence.
    expect(previewErrorText(new Error("An dieser Stelle liegt keine Karte."))).toBe("An dieser Stelle liegt keine Karte.");
  });

  it("in English, the bridge's and the policy's English refusals keep their text, any other error says in English what to check", async () => {
    await i18next.changeLanguage("en");
    try {
      const sentence = "The preview picture could not be changed. Check that the project folder is still there and that you may write in it.";
      expect(previewErrorText(refused("Permission denied (os error 13)"))).toBe(sentence);
      for (const own of [
        "Path is outside the allowed folders: /tmp/x.png",
        "Path is not valid text: /x",
        "Invalid arguments for project_preview_set: y",
        "Path not readable: /x.png: No such file",
        "Folder not readable: /x: Permission denied",
        "Not a file: /x",
        "Not a folder: /x",
        "No file name: /",
        "Originals (_DSPRE_contents) are locked: /p_DSPRE_contents/x.png",
        "Originals are locked, only copies under /k: /o/x.png",
        "Save games (_saves) are locked: /p/_saves/x.png",
        "Only the open project: /q/x.png",
        "Bridge not reachable",
        "No Tauri and no bridge",
      ]) {
        expect(previewErrorText(refused(own))).toBe(own);
      }
      // A German refusal (from a bridge still speaking German) keeps its text too.
      expect(previewErrorText(refused("Pfad liegt außerhalb der erlaubten Ordner: /tmp/x.png"))).toBe("Pfad liegt außerhalb der erlaubten Ordner: /tmp/x.png");
    } finally {
      await i18next.changeLanguage("de");
    }
  });

  it("change_preview_says_a_failed_write_in_german", async () => {
    vi.mocked(projectPreviewSet).mockRejectedValue(refused("Permission denied (os error 13)"));
    await expect(changePreview("remove", "/p")).resolves.toBeNull();
    expect(toasts()).toEqual([["error", "Das Vorschaubild ließ sich nicht ändern. Prüfe, ob der Projektordner noch da ist und du darin schreiben darfst."]]);
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
