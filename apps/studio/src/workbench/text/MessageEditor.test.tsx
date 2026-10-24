// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { validateText } from "../../api";
import { MessageEditor } from "./MessageEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), validateText: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("MessageEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn(async () => true);
  const onDraft = vi.fn();
  const onFocusChange = vi.fn();

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    for (const f of [onCommit, onDraft, onFocusChange]) f.mockClear();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  const field = () => container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Text der Nachricht"]')!;
  const show = (value: string) => act(() => root.render(<MessageEditor value={value} onCommit={onCommit} onDraft={onDraft} onFocusChange={onFocusChange} />));
  const shiftEnter = () => act(() => void field().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", shiftKey: true, bubbles: true, cancelable: true })));

  it("inserts the clear code at the caret for Shift+Enter and keeps the caret behind it", () => {
    show("EinsZwei");
    field().setSelectionRange(4, 4);
    shiftEnter();
    expect(field().value).toBe("Eins\\rZwei");
    expect(field().selectionStart).toBe(6);
    expect(onDraft).toHaveBeenLastCalledWith("Eins\\rZwei");
  });

  it("replaces a selection with the clear code and commits it after the rest", async () => {
    show("Eins und Zwei");
    vi.useFakeTimers();
    field().setSelectionRange(4, 9);
    shiftEnter();
    expect(field().value).toBe("Eins\\rZwei");
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(onCommit).toHaveBeenCalledWith("Eins\\rZwei");
  });

  it("says when it gets and loses the focus", () => {
    show("Text");
    act(() => field().focus());
    expect(onFocusChange).toHaveBeenLastCalledWith(true);
    act(() => void field().dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(onFocusChange).toHaveBeenLastCalledWith(false);
  });
});
