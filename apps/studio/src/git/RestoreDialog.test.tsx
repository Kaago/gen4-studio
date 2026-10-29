// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { restoreTextProject } from "../api";
import type { RestoreProgress } from "../api";
import { openPath } from "../platform/dialogs";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { pressKey } from "../ui/primitives/testUtils";
import { RestoreDialog } from "./RestoreDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  restoreTextProject: vi.fn(),
}));
vi.mock("../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../platform/dialogs")>()), openPath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MESSAGE = "Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein frischer Git-Klon. Basis-ROM: Platin DE (Testprojekt).";
const MISSING = "Die Basis-ROM „Platin DE (Testprojekt)“ ist nicht da, wo sie zuletzt war.";

describe("RestoreDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onOpened = vi.fn();

  const show = () => act(async () => root.render(<RestoreDialog root="/k/klon" message={MESSAGE} onClose={onClose} onOpened={onOpened} />));
  const dialog = () => document.querySelector<HTMLElement>("[data-restore-dialog]");
  const buttons = () => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].filter((b) => b.textContent !== "");
  const labels = () => buttons().map((b) => b.textContent);
  const button = (text: string) => buttons().find((b) => b.textContent === text);
  const progress = () => dialog()?.querySelector("[data-restore-progress]")?.textContent ?? null;

  beforeEach(() => {
    onClose.mockReset();
    onOpened.mockReset();
    resetToasts();
    vi.mocked(restoreTextProject).mockReset();
    vi.mocked(openPath).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
  });

  it("restores_then_opens", async () => {
    let report: (p: RestoreProgress) => void = () => {};
    let finish: () => void = () => {};
    vi.mocked(restoreTextProject).mockImplementationOnce((_root, _nds, onProgress) => {
      report = onProgress;
      return new Promise((resolve) => (finish = () => resolve({ moved: 11, ms: 52_800 })));
    });
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Textprojekt vervollständigen?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      `${MESSAGE} Gen4 Studio legt die Spieldateien aus deiner Basis-ROM an; deine Dateien bleiben, wie sie sind.`,
    );
    expect(labels()).toEqual(["Abbrechen", "Vervollständigen"]);
    expect(button("Vervollständigen")?.dataset.kind).toBe("primary");

    await act(async () => button("Vervollständigen")!.click());
    expect(restoreTextProject).toHaveBeenCalledExactlyOnceWith("/k/klon", null, expect.any(Function));
    // While it runs there is no button, and Escape does not close it.
    expect(labels()).toEqual([]);
    pressKey(dialog()!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
    expect(dialog()?.querySelector("[data-restore-progress]")?.getAttribute("role")).toBe("status");
    const seen: (string | null)[] = [progress()];
    for (const p of [
      { phase: "extract", done: 0, total: 1 },
      { phase: "merge", done: 0, total: 1 },
      { phase: "import", done: 3, total: 17 },
      { phase: "sync", done: 0, total: 1 },
    ] as RestoreProgress[]) {
      await act(async () => report(p));
      seen.push(progress());
    }
    expect(seen).toEqual(["Basis-ROM auspacken …", "Basis-ROM auspacken …", "Spieldateien einsetzen …", "Textprojekt einspielen … 3 von 17", "Abgleich vorbereiten …"]);

    await act(async () => finish());
    expect(onOpened).toHaveBeenCalledOnce();
    expect(listToasts().map((t) => `${t.kind}: ${t.text}`)).toEqual(["info: Textprojekt vervollständigt in 53 s."]);

    // „Abbrechen“ closes without anything done.
    await show();
    await act(async () => button("Abbrechen")!.click());
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("without_a_base_offers_to_choose_one", async () => {
    vi.mocked(restoreTextProject).mockRejectedValueOnce({ code: "base_missing", message: MISSING, asset: null });
    await show();
    await act(async () => button("Vervollständigen")!.click());
    // The reason as a warning, with the way out: choosing the base ROM.
    const warning = [...(dialog()?.querySelectorAll<HTMLElement>("[role=status]") ?? [])].find((n) => n.textContent?.startsWith(MISSING));
    expect(warning).toBeDefined();
    expect(warning?.querySelector("button")?.textContent).toBe("Basis-ROM wählen…");
    expect(onOpened).not.toHaveBeenCalled();

    // A foreign ROM: the kernel's sentence, and the choice stays.
    const other = "Diese ROM ist nicht die Basis-ROM dieses Projekts.";
    vi.mocked(openPath).mockResolvedValueOnce("/roms/hg.nds");
    vi.mocked(restoreTextProject).mockRejectedValueOnce({ code: "error", message: other, asset: null });
    await act(async () => button("Basis-ROM wählen…")!.click());
    expect(openPath).toHaveBeenLastCalledWith({ directory: false, title: "Basis-ROM wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] });
    expect(restoreTextProject).toHaveBeenLastCalledWith("/k/klon", "/roms/hg.nds", expect.any(Function));
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe(other);
    expect(button("Basis-ROM wählen…")).toBeDefined();

    // Nothing chosen: nothing runs.
    vi.mocked(openPath).mockResolvedValueOnce(null);
    await act(async () => button("Basis-ROM wählen…")!.click());
    expect(restoreTextProject).toHaveBeenCalledTimes(2);

    // The right ROM: completed and opened.
    vi.mocked(openPath).mockResolvedValueOnce("/roms/pt.nds");
    vi.mocked(restoreTextProject).mockResolvedValueOnce({ moved: 11, ms: 40_000 });
    await act(async () => button("Basis-ROM wählen…")!.click());
    expect(restoreTextProject).toHaveBeenLastCalledWith("/k/klon", "/roms/pt.nds", expect.any(Function));
    expect(onOpened).toHaveBeenCalledOnce();
  });
});
