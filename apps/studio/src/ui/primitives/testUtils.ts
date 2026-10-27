// Helpers shared by the primitive tests: typing into inputs and pressing keys the way a browser delivers them.
import { act } from "react";
import { vi } from "vitest";

export function typeInto(el: HTMLInputElement | HTMLTextAreaElement, text: string): void {
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  act(() => {
    Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(el, text);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

export function pressKey(el: HTMLElement, key: string, init: KeyboardEventInit = {}): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  act(() => {
    el.dispatchEvent(event);
  });
  return event;
}

/** Opens a Base UI select the way a pointer does (mousedown, mouseup, click on the trigger). */
export function openSelect(trigger: HTMLElement): void {
  act(() => {
    trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
    trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
    trigger.click();
  });
}

/** Types into a picker's search field as a user does: focused, with an `inputType`, so the first match is highlighted. */
export function typeInSearch(el: HTMLInputElement, text: string): void {
  act(() => {
    el.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(el, text);
    el.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: text }));
  });
}

/**
 * A page whose animation frames do not come (a hidden one, a background tab; a busy one has them late) and whose popups animate, as in a
 * browser: Base UI waits for both before it moves the focus or reports an open menu complete. Returns the undo.
 */
export function withoutFrames(): () => void {
  const proto = Element.prototype as { getAnimations?: () => Animation[] };
  const had = proto.getAnimations;
  vi.stubGlobal("requestAnimationFrame", () => 0);
  proto.getAnimations = () => [];
  return () => {
    vi.unstubAllGlobals();
    if (had) proto.getAnimations = had;
    else delete proto.getAnimations;
  };
}
