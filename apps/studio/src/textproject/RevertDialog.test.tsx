// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { revertTextProject } from "../api";
import type { RevertReport } from "../api";
import { RevertDialog, revertedText } from "./RevertDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  revertTextProject: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const PENDING = "Erst abgleichen: es gibt Änderungen von außen, Konflikte oder fehlerhafte Dateien (Technik › Textprojekt).";
const report: RevertReport = { backup: "/p/.g4/backup/textproject-20261028T120000Z", rewritten: ["/p/g4-world.json"], ms: 40 };

describe("RevertDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onReverted = vi.fn();

  const show = (unsaved: number) => act(async () => root.render(<RevertDialog unsaved={unsaved} onClose={onClose} onReverted={onReverted} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);
  const notices = (role: "status" | "alert") => [...(dialog()?.querySelectorAll<HTMLElement>(`[role=${role}]`) ?? [])].map((n) => n.textContent);

  beforeEach(() => {
    onClose.mockReset();
    onReverted.mockReset();
    vi.mocked(revertTextProject).mockReset().mockResolvedValue(report);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("unsaved_locks_and_pending_shows_the_kernel_sentence", async () => {
    await show(1);
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Textprojekt zurücknehmen?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      "Das Studio arbeitet danach wieder direkt mit den DSPRE-Dateien. g4-project.json und data/ wandern nach .g4/backup/; die Spieldateien bleiben, wie sie sind.",
    );
    expect(notices("status")).toEqual(["Erst speichern."]);
    expect(button("Zurücknehmen")?.disabled).toBe(true);
    await act(async () => button("Zurücknehmen")!.click());
    expect(revertTextProject).not.toHaveBeenCalled();

    // Saved: it can be taken back; the kernel refuses while something waits for the sync and says so.
    await show(0);
    expect(notices("status")).toEqual([]);
    vi.mocked(revertTextProject).mockRejectedValueOnce({ code: "pending", message: PENDING, asset: null });
    await act(async () => button("Zurücknehmen")!.click());
    expect(notices("alert")).toEqual([PENDING]);
    expect(onReverted).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();

    // Once more, now it goes through: the dialog closes and the message names the backup.
    await act(async () => button("Zurücknehmen")!.click());
    expect(onReverted).toHaveBeenCalledWith(report);
    expect(onClose).toHaveBeenCalled();
    expect(revertedText(report)).toBe("Textprojekt zurückgenommen. Sicherung: /p/.g4/backup/textproject-20261028T120000Z");
    // „Abbrechen“ only closes.
    onClose.mockClear();
    await show(0);
    await act(async () => button("Abbrechen")!.click());
    expect(onClose).toHaveBeenCalled();
  });
});
