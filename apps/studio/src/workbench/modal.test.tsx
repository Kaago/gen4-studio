// @vitest-environment jsdom
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { escapeCloses, useFocusTrap } from "./modal";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** A keydown for `target` whose `defaultPrevented` is as given (dispatching is not needed for the rule). */
function key(target: EventTarget, name = "Escape", prevented = false): KeyboardEvent {
  const e = new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true });
  Object.defineProperty(e, "target", { value: target });
  if (prevented) e.preventDefault();
  return e;
}

describe("escapeCloses", () => {
  const input = (type: string, value = "", draft = false) => {
    const el = document.createElement("input");
    el.type = type;
    el.value = value;
    if (draft) el.setAttribute("data-draft", "");
    return el;
  };

  it("closes_from_buttons_radios_empty_fields_and_unedited_draft_fields", () => {
    expect(escapeCloses(key(document))).toBe(true);
    expect(escapeCloses(key(document.createElement("button")))).toBe(true);
    expect(escapeCloses(key(input("radio")))).toBe(true);
    expect(escapeCloses(key(input("search")))).toBe(true);
    expect(escapeCloses(key(input("number", "12", true)))).toBe(true);
  });

  it("does_not_close_when_the_key_was_used_or_a_field_still_holds_text", () => {
    expect(escapeCloses(key(input("number", "12", true), "Escape", true))).toBe(false);
    expect(escapeCloses(key(input("search", "Zweiblatt")))).toBe(false);
    expect(escapeCloses(key(document.createElement("select")))).toBe(false);
    expect(escapeCloses(key(document.createElement("textarea")))).toBe(false);
    expect(escapeCloses(key(document, "Enter"))).toBe(false);
  });
});

describe("useFocusTrap", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function Dialog() {
    const box = useRef<HTMLDivElement>(null);
    useFocusTrap(box);
    return (
      <div ref={box}>
        <button>eins</button>
        <button disabled>aus</button>
        <button>zwei</button>
      </div>
    );
  }
  const tab = (shift = false) => act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: shift, bubbles: true, cancelable: true })));
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text)!;

  it("wraps_tab_and_shift_tab_inside_and_gives_the_focus_back_on_unmount", () => {
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.focus();
    act(() => root.render(<Dialog />));
    button("zwei").focus();
    tab();
    expect(document.activeElement).toBe(button("eins"));
    tab(true);
    expect(document.activeElement).toBe(button("zwei"));
    act(() => root.render(<></>));
    expect(document.activeElement).toBe(outside);
    outside.remove();
  });

  it("pulls_a_focus_from_outside_back_in", () => {
    const outside = document.createElement("button");
    document.body.append(outside);
    act(() => root.render(<Dialog />));
    outside.focus();
    tab();
    expect(document.activeElement).toBe(button("eins"));
    outside.remove();
  });
});
