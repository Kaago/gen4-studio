// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DraftInput } from "./DraftInput";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DraftInput", () => {
  let container: HTMLElement;
  let root: Root;
  const commits = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    commits.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** An owner that takes every commit and can drop the field, as a wizard step does on "Weiter". */
  function Owner() {
    const [value, setValue] = useState("Neues Gebiet");
    const [shown, setShown] = useState(true);
    return (
      <div>
        {shown && (
          <DraftInput
            label="Name"
            value={value}
            onCommit={(text) => {
              commits(text);
              setValue(text);
            }}
          />
        )}
        <span data-value>{value}</span>
        <button type="button" onClick={() => setShown(false)}>
          Weiter
        </button>
      </div>
    );
  }
  const input = () => container.querySelector("input")!;
  const typeText = (text: string) => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input(), text);
    input().dispatchEvent(new Event("input", { bubbles: true }));
  };

  it("commits_on_blur_and_shows_the_owners_value", () => {
    act(() => root.render(<Owner />));
    act(() => input().focus());
    act(() => typeText("Route 231"));
    act(() => input().blur());
    expect(commits.mock.calls).toEqual([["Route 231"]]);
    expect(input().value).toBe("Route 231");
  });

  it("commits_typed_text_when_it_unmounts_without_a_blur", () => {
    act(() => root.render(<Owner />));
    act(() => typeText("Route 231"));
    act(() => container.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(commits.mock.calls).toEqual([["Route 231"]]);
    expect(container.querySelector("[data-value]")?.textContent).toBe("Route 231");
  });

  it("does_not_commit_again_when_it_unmounts_right_after_the_blur", () => {
    // Gate 3b.1 (A) 6: "Weiter" right after typing; blur and click land before the field shows the new value, and
    // the unmount committed a draft that was no longer the typed text.
    act(() => root.render(<Owner />));
    act(() => input().focus());
    act(() => typeText("Route 231"));
    act(() => {
      input().blur();
      container.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(commits.mock.calls).toEqual([["Route 231"]]);
    expect(container.querySelector("[data-value]")?.textContent).toBe("Route 231");
  });

  it("takes_back_the_typed_text_with_escape", () => {
    act(() => root.render(<Owner />));
    act(() => input().focus());
    act(() => typeText("Route 231"));
    act(() => void input().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(input().value).toBe("Neues Gebiet");
    act(() => input().blur());
    expect(commits).not.toHaveBeenCalled();
  });

  it("uses_enter_only_while_it_holds_typed_text", () => {
    // A wizard around the field moves on with an Enter the field left alone, never with the one that commits.
    const enter = () => {
      const event = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
      act(() => void input().dispatchEvent(event));
      return event;
    };
    act(() => root.render(<Owner />));
    act(() => input().focus());
    expect(enter().defaultPrevented).toBe(false);
    act(() => input().focus());
    act(() => typeText("Route 231"));
    expect(enter().defaultPrevented).toBe(true);
    expect(commits.mock.calls).toEqual([["Route 231"]]);
  });
});
