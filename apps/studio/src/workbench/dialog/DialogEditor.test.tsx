// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogOf, editTextArchive, splitMessage, validateText } from "../../api";
import type { Dialog, DialogMessage } from "../../api";
import { pressKey } from "../../ui/primitives/testUtils";
import type { DialogTarget } from "./dialogModel";
import { DialogEditor } from "./DialogEditor";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dialogOf: vi.fn(),
  editTextArchive: vi.fn(),
  splitMessage: vi.fn(),
  validateText: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const figure: DialogTarget = { header: 3, eventFile: 8, kind: "object", index: 2, title: "Objekt 2" };
const message = (index: number, text: string, over: Partial<DialogMessage> = {}): DialogMessage => ({ archive: 554, index, text, command: "Message", line: 8, col: 13, branch: null, shared_with: 0, ...over });
const dialog = (messages: DialogMessage[]): Dialog => ({ source: { kind: "local", file: 12, slot: 4, line: 7 }, messages, followed: [] });

describe("DialogEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onChanged = vi.fn();
  const onError = vi.fn();
  const scriptFiles = { saveIfDirty: vi.fn(async () => {}), reload: vi.fn(async () => {}) };
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(dialogOf).mockReset().mockResolvedValue(dialog([message(10, String.raw`Hallo!\rSchönes Wetter.`), message(11, "Tschüss.", { shared_with: 2, branch: "CheckFlag 100" })]));
    vi.mocked(editTextArchive).mockReset().mockResolvedValue(7);
    vi.mocked(splitMessage).mockReset();
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    for (const fn of [onClose, onChanged, onError, scriptFiles.saveIfDirty, scriptFiles.reload]) fn.mockClear();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show() {
    await act(async () => root.render(<DialogEditor target={figure} revision={0} onClose={onClose} onChanged={onChanged} onError={onError} scriptFiles={scriptFiles} />));
    await flush();
  }
  const editor = () => document.querySelector<HTMLElement>("[data-dialog-editor]");
  const block = (key: string) => document.querySelector<HTMLElement>(`[data-dialog-message="${key}"]`);
  const field = (key: string) => block(key)!.querySelector<HTMLTextAreaElement>("textarea")!;
  const button = (text: string, within: ParentNode = document) => [...within.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === text);
  const screen = () => editor()!.querySelector<HTMLElement>('[role="group"][aria-label^="DS-Bildschirm"]');
  function type(el: HTMLTextAreaElement, value: string) {
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(el, value);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
  }
  const blur = (el: HTMLElement) => act(async () => void el.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));

  it("edits_one_message_through_edit_text_archive_with_the_label", async () => {
    await show();
    expect(dialogOf).toHaveBeenCalledWith(3, 8, "object", 2);
    expect(editor()?.querySelector("h2")?.textContent).toBe("Dialog von Objekt 2");
    expect(block("554:10")?.textContent).toContain("Nachricht 10 · Archiv 554");
    expect(block("554:11")?.textContent).toContain("Nachricht 11 · Archiv 554 · wenn CheckFlag 100");
    // The screen shows the first message, page by page.
    expect(screen()?.textContent).toContain("Hallo!");
    expect(editor()?.textContent).toContain("Seite 1 von 2");
    await act(async () => button("Nächste Seite")!.click());
    expect(editor()?.textContent).toContain("Seite 2 von 2");
    expect(screen()?.textContent).toContain("Schönes Wetter.");
    type(field("554:10"), "Guten Tag!");
    // The screen follows the typing.
    expect(screen()?.textContent).toContain("Guten Tag!");
    await blur(field("554:10"));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(554, { Set: { index: 10, text: "Guten Tag!" } }, "Dialog geändert");
    expect(onChanged).toHaveBeenCalledOnce();
    // The editor keeps no text: it reads the dialog again.
    expect(dialogOf).toHaveBeenCalledTimes(2);
    // Selecting another message shows it on the screen.
    await act(async () => block("554:11")!.querySelector("button")!.click());
    expect(screen()?.textContent).toContain("Tschüss.");
    expect(editor()?.textContent).toContain("Seite 1 von 1");
  });

  it("a_shared_message_shows_the_badge_and_splits", async () => {
    await show();
    expect(block("554:10")?.textContent).not.toContain("Geteilt");
    expect(button("Für diese Figur abspalten", block("554:10")!)).toBeUndefined();
    expect(block("554:11")?.textContent).toContain("Geteilt mit 2 Figuren");
    vi.mocked(splitMessage).mockResolvedValue({ archive: 554, old_index: 11, new_index: 40, change: 5, lines: [9] });
    vi.mocked(dialogOf).mockResolvedValue(dialog([message(10, "Hallo!"), message(40, "Tschüss.")]));
    await act(async () => button("Für diese Figur abspalten", block("554:11")!)!.click());
    await flush();
    expect(splitMessage).toHaveBeenCalledWith(3, 8, 2, 554, 11);
    // The script file is saved first when it has typing and shown again after the backend wrote it.
    expect(scriptFiles.saveIfDirty).toHaveBeenCalledWith(12);
    expect(scriptFiles.reload).toHaveBeenCalledWith(12);
    expect(onChanged).toHaveBeenCalledOnce();
    expect(editor()?.textContent).toContain("Nachricht 40 ist jetzt nur diese Figur.");
    expect(block("554:40")).not.toBeNull();
    expect(block("554:40")?.textContent).not.toContain("Geteilt");
  });

  it("a_refused_text_shows_the_notice", async () => {
    await show();
    vi.mocked(editTextArchive).mockRejectedValue({ code: "error", message: "Archiv 554 ist gesperrt.", asset: null });
    type(field("554:10"), "Neu");
    await blur(field("554:10"));
    await flush();
    expect(editor()?.querySelector("[role=alert]")?.textContent).toContain("Archiv 554 ist gesperrt.");
    expect(onChanged).not.toHaveBeenCalled();
    // The field shows the stored text again.
    expect(field("554:10").value).toBe(String.raw`Hallo!\rSchönes Wetter.`);
    // A refused split says the kernel's sentence the same way.
    vi.mocked(splitMessage).mockRejectedValue({ code: "error", message: "Die Nachricht steht in einem gemeinsamen Sprungziel; in Bauen im Script trennen.", asset: null });
    await act(async () => button("Für diese Figur abspalten", block("554:11")!)!.click());
    await flush();
    expect(editor()?.querySelector("[role=alert]")?.textContent).toContain("gemeinsamen Sprungziel");
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("esc_closes", async () => {
    await show();
    expect([...editor()!.querySelectorAll("button")].some((b) => b.textContent === "Schließen")).toBe(true);
    pressKey((document.activeElement as HTMLElement) ?? editor()!, "Escape");
    expect(onClose).toHaveBeenCalled();
  });
});
