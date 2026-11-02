// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { editTextArchive, splitMessage, validateText } from "../../api";
import type { Dialog, DialogMessage } from "../../api";
import { distinctMessages } from "./dialogModel";
import type { DialogTarget } from "./dialogModel";
import { DialogMessages, useDialogWrites } from "./DialogMessages";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  editTextArchive: vi.fn(),
  splitMessage: vi.fn(),
  validateText: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const target: DialogTarget = { header: 3, eventFile: 8, kind: "object", index: 2, title: "Objekt 2" };
const message = (index: number, text: string | null, over: Partial<DialogMessage> = {}): DialogMessage => ({ archive: 554, index, text, command: "Message", line: 8, col: 13, branch: null, shared_with: 0, split_refusal: null, ...over });
const dialog = (messages: DialogMessage[]): Dialog => ({ source: { kind: "local", file: 12, slot: 4, line: 7 }, messages, followed: [] });

describe("DialogMessages", () => {
  let container: HTMLElement;
  let root: Root;
  const reload = vi.fn();
  const onChanged = vi.fn();
  const onError = vi.fn();
  const onDraft = vi.fn();
  const scriptFiles = { saveIfDirty: vi.fn(async () => {}), reload: vi.fn(async () => {}) };

  /** The messages as the hand and the editor use them: the writes of the figure and the chosen message. */
  function Host({ d, kind = "object" }: { d: Dialog; kind?: "object" | "bg" }) {
    const [chosen, setChosen] = useState<string | null>(null);
    const writes = useDialogWrites({ ...target, kind }, d.source, { reload, onChanged, onError, scriptFiles, onSplit: setChosen });
    const distinct = distinctMessages(d.messages);
    return (
      <>
        {writes.notice !== null && <p data-notice="">{writes.notice.text}</p>}
        <DialogMessages kind={kind} distinct={distinct} currentKey={chosen ?? "554:10"} onChoose={setChosen} onDraft={onDraft} writes={writes} />
      </>
    );
  }
  const show = (d: Dialog, kind: "object" | "bg" = "object") => act(() => root.render(<Host d={d} kind={kind} />));
  const block = (key: string) => container.querySelector<HTMLElement>(`[data-dialog-message="${key}"]`);
  const button = (text: string, within: ParentNode = container) => [...within.querySelectorAll("button")].find((b) => b.textContent === text);

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(editTextArchive).mockReset().mockResolvedValue(7);
    vi.mocked(splitMessage).mockReset();
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    for (const fn of [reload, onChanged, onError, onDraft, scriptFiles.saveIfDirty, scriptFiles.reload]) fn.mockClear();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("lists each message once with its head, the shared badge and the field", () => {
    show(dialog([message(10, "Hallo!"), message(11, "Tschüss.", { shared_with: 2, branch: "CheckFlag 100" }), message(10, "Hallo!", { line: 12 })]));
    expect(container.querySelector('ol[aria-label="Nachrichten"]')).not.toBeNull();
    expect(container.querySelectorAll("[data-dialog-message]")).toHaveLength(2);
    expect(block("554:10")?.textContent).toContain("Nachricht 10 · Archiv 554 · auch Zeile 12");
    expect(block("554:10")?.hasAttribute("data-selected")).toBe(true);
    expect(block("554:11")?.textContent).toContain("wenn CheckFlag 100");
    expect(block("554:11")?.textContent).toContain("Geteilt mit 2 Figuren");
    expect(button("Für diese Figur abspalten", block("554:11")!)).toBeDefined();
    expect(button("Für diese Figur abspalten", block("554:10")!)).toBeUndefined();
    expect(block("554:10")?.querySelector("textarea")?.value).toBe("Hallo!");
  });

  it("a sign's shared message has no split, a missing message says so", () => {
    show(dialog([message(10, "Hallo!", { shared_with: 1 }), message(30, null)]), "bg");
    expect(button("Für diese Figur abspalten")).toBeUndefined();
    expect(block("554:30")?.textContent).toContain("Archiv 554 hat keine Nachricht 30.");
  });

  it("commits a typed text with the label of the dialog and reads the dialog again", async () => {
    show(dialog([message(10, "Hallo!")]));
    const field = block("554:10")!.querySelector("textarea")!;
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(field, "Guten Tag!");
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(onDraft).toHaveBeenLastCalledWith("554:10", "Guten Tag!");
    await act(async () => void field.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(554, { Set: { index: 10, text: "Guten Tag!" } }, "Dialog geändert");
    expect(onChanged).toHaveBeenCalledOnce();
    expect(reload).toHaveBeenCalledOnce();
  });

  it("a refused text says why", async () => {
    vi.mocked(editTextArchive).mockRejectedValue({ code: "error", message: "Archiv gesperrt", asset: null });
    show(dialog([message(10, "Hallo!")]));
    const field = block("554:10")!.querySelector("textarea")!;
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(field, "Neu");
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => void field.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await flush();
    expect(container.querySelector("[data-notice]")?.textContent).toContain("Archiv gesperrt");
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("splits a shared message for this figure and chooses the copy", async () => {
    vi.mocked(splitMessage).mockResolvedValue({ archive: 554, old_index: 11, new_index: 40, change: 5, lines: [9] });
    show(dialog([message(10, "Hallo!"), message(11, "Tschüss.", { shared_with: 1 })]));
    await act(async () => button("Für diese Figur abspalten", block("554:11")!)!.click());
    await flush();
    expect(scriptFiles.saveIfDirty).toHaveBeenCalledWith(12);
    expect(splitMessage).toHaveBeenCalledWith(3, 8, 2, 554, 11);
    expect(scriptFiles.reload).toHaveBeenCalledWith(12);
    expect(onChanged).toHaveBeenCalledOnce();
    expect(reload).toHaveBeenCalledOnce();
    expect(container.querySelector("[data-notice]")?.textContent).toBe("Nachricht 40 ist jetzt nur diese Figur.");
  });
});
