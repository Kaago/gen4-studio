// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Notice } from "./Notice";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Notice", () => {
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

  it.each(["info", "success", "warning"] as const)("%s_is_a_status", (kind) => {
    act(() => root.render(<Notice kind={kind}>Text</Notice>));
    expect(container.querySelector("[role=status]")?.textContent).toContain("Text");
    expect(container.querySelector("[role=alert]")).toBeNull();
  });

  it("error_is_an_alert", () => {
    act(() => root.render(<Notice kind="error">Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür.</Notice>));
    expect(container.querySelector("[role=alert]")?.textContent).toContain("Die Tür führt ins Leere.");
    expect(container.querySelector("[role=status]")).toBeNull();
  });

  it("shows_the_title_before_the_body", () => {
    act(() => root.render(<Notice kind="warning" title="Nicht rückgängig">Das lässt sich nicht zurücknehmen.</Notice>));
    const text = container.querySelector("[role=status]")!.textContent ?? "";
    expect(text.indexOf("Nicht rückgängig")).toBeLessThan(text.indexOf("Das lässt sich"));
  });

  it("renders_actions_as_buttons_that_call_back", () => {
    const first = vi.fn();
    const second = vi.fn();
    act(() =>
      root.render(
        <Notice
          kind="error"
          actions={[
            { label: "Probleme zeigen", onClick: first },
            { label: "Erneut versuchen", onClick: second },
          ]}
        >
          Build abgebrochen
        </Notice>,
      ),
    );
    const buttons = [...container.querySelectorAll("button")];
    expect(buttons.map((b) => b.textContent)).toEqual(["Probleme zeigen", "Erneut versuchen"]);
    act(() => buttons[1]!.click());
    expect(second).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
  });
});
