// Helpers shared by the primitive tests: typing into inputs and pressing keys the way a browser delivers them.
import { act } from "react";

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
