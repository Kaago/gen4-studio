// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { adoptTextProject } from "../api";
import type { AdoptProgress, AdoptReport } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { AdoptDialog, adoptedText } from "./AdoptDialog";
import type { AdoptDialogProps } from "./AdoptDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  adoptTextProject: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const report: AdoptReport = {
  export: {
    arts: [],
    raw_blobs: 0,
    raw_bytes: 0,
    raw_ms: 0,
    not_built: [],
    texts: 0,
    text_json_written: 0,
    scripts: 0,
    adopted: null,
    registries_written: 4,
    base_layer_ms: 0,
    base_layer_created: true,
    files: 6512,
    bytes: 1,
    written: 6512,
    removed: 0,
    problems: [],
    ms: 80000,
  },
  include_written: true,
  sync_files: 9000,
  ms: 88400,
};

describe("AdoptDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onAdopted = vi.fn();

  const show = (over: Partial<AdoptDialogProps> = {}) =>
    act(async () => root.render(<AdoptDialog unsaved={0} baseLabel="Platin DE (Testprojekt)" baseReason={null} scriptsAdopted={false} onClose={onClose} onAdopted={onAdopted} {...over} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])];
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const rows = () => [...(dialog()?.querySelectorAll<HTMLElement>("[data-check]") ?? [])].map((r) => `${r.dataset.state}: ${r.textContent}`);
  const status = () => dialog()?.querySelector("[data-adopt-progress]")?.textContent ?? null;

  beforeEach(() => {
    onClose.mockReset();
    onAdopted.mockReset();
    vi.mocked(adoptTextProject).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("checklist_then_progress_in_order_then_result", async () => {
    let progress: (p: AdoptProgress) => void = () => {};
    let finish: (r: AdoptReport) => void = () => {};
    vi.mocked(adoptTextProject).mockImplementation((onProgress) => {
      progress = onProgress;
      return new Promise((resolve) => (finish = resolve));
    });
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Als Textprojekt einrichten?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      "Schreibt g4-project.json und rund 6 500 Dateien unter data/. Die Spieldateien bleiben, wie sie sind; zurücknehmen geht in Technik › Textprojekt.",
    );
    expect(rows()).toEqual(["ok: Alles gespeichert", "ok: Basis-ROM gefunden: Platin DE (Testprojekt)", "info: Scripts werden dabei übernommen"]);
    expect(buttons().filter((b) => b.textContent !== "").map((b) => b.textContent)).toEqual(["Abbrechen", "Einrichten"]);

    await act(async () => button("Einrichten")!.click());
    expect(adoptTextProject).toHaveBeenCalledOnce();
    // While it runs there is no button at all, and Escape does not close it.
    expect(buttons()).toEqual([]);
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
    const seen: (string | null)[] = [];
    for (const p of [
      { phase: "scripts", done: 0, total: 3 },
      { phase: "scripts", done: 2, total: 3 },
      { phase: "base_layer", done: 0, total: 1 },
      { phase: "entities", done: 4, total: 17 },
      { phase: "sync", done: 0, total: 9000 },
    ] as AdoptProgress[]) {
      act(() => progress(p));
      seen.push(status());
    }
    expect(seen).toEqual([
      "Scripts übernehmen … 0 von 3",
      "Scripts übernehmen … 2 von 3",
      "Basisschicht anlegen …",
      "Einträge schreiben … 4 von 17 Arten",
      "Abgleich vorbereiten …",
    ]);
    expect(dialog()?.querySelector("[data-adopt-progress]")?.getAttribute("role")).toBe("status");

    await act(async () => finish(report));
    expect(onAdopted).toHaveBeenCalledWith(report);
    expect(onClose).toHaveBeenCalled();
    // UI test 4.3b U7: no line breaks between a count and its unit.
    expect(adoptedText(report)).toBe("Textprojekt eingerichtet: 6.512\u00a0Dateien in 88\u00a0s.");
  });

  it("unsaved_locks_set_up", async () => {
    await show({ unsaved: 2, scriptsAdopted: true });
    expect(rows()).toEqual(["error: Erst speichern: 2 ungespeicherte Änderungen.", "ok: Basis-ROM gefunden: Platin DE (Testprojekt)", "ok: Scripts übernommen"]);
    expect(button("Einrichten")?.disabled).toBe(true);
    await act(async () => button("Einrichten")!.click());
    expect(adoptTextProject).not.toHaveBeenCalled();
    // Without the base ROM it is locked as well, with the reason.
    await show({ baseLabel: null, baseReason: "Diesem Projekt ist keine Basis-ROM zugeordnet." });
    expect(rows()[1]).toBe("error: Diesem Projekt ist keine Basis-ROM zugeordnet.");
    expect(button("Einrichten")?.disabled).toBe(true);
    // A refusal of the backend shows its sentence, and the dialog can try again.
    await show();
    vi.mocked(adoptTextProject).mockRejectedValueOnce({ code: "unsaved", message: "Erst speichern: 1 ungespeicherte Änderung.", asset: null });
    await act(async () => button("Einrichten")!.click());
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("Erst speichern: 1 ungespeicherte Änderung.");
    expect(button("Einrichten")?.disabled).toBe(false);
    expect(onAdopted).not.toHaveBeenCalled();
    // The files moved but the session changed meanwhile: the sentence says to open the project anew; only closing is left.
    vi.mocked(adoptTextProject).mockRejectedValueOnce({ code: "error", message: "Die Dateien sind umgestellt, aber die offene Sitzung wurde inzwischen geändert und bleibt, wie sie ist; das Projekt bitte neu öffnen.", asset: null });
    await act(async () => button("Einrichten")!.click());
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toContain("das Projekt bitte neu öffnen");
    expect(buttons().filter((b) => b.textContent !== "").map((b) => b.textContent)).toEqual(["Schließen"]);
  });
});
