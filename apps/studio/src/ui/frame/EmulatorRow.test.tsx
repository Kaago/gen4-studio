// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EmulatorRow } from "./EmulatorRow";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("EmulatorRow", () => {
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
  const pick = () => [...container.querySelectorAll("button")].find((b) => b.textContent === "Emulator wählen");

  it("says_that_no_emulator_is_chosen_for_null_and_undefined", () => {
    for (const path of [null, undefined]) {
      act(() => root.render(<EmulatorRow path={path} onPick={() => {}} />));
      expect(container.textContent).toContain("Kein Emulator gewählt");
    }
  });

  it("shows_the_path_with_it_as_tooltip", () => {
    act(() => root.render(<EmulatorRow path="/Applications/melonDS.app" onPick={() => {}} />));
    expect(container.textContent).toContain("/Applications/melonDS.app");
    expect(container.textContent).not.toContain("Kein Emulator gewählt");
    expect(container.querySelector('[title="/Applications/melonDS.app"]')).not.toBeNull();
  });

  it("the_button_asks_for_an_emulator", () => {
    const onPick = vi.fn();
    act(() => root.render(<EmulatorRow path={null} onPick={onPick} />));
    act(() => pick()!.click());
    expect(onPick).toHaveBeenCalledOnce();
  });
});
