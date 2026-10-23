// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ToastHost } from "./ToastHost";
import { INFO_MS, resetToasts, toast } from "./toasts";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ToastHost", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    vi.useFakeTimers();
    resetToasts();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<ToastHost />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
    vi.useRealTimers();
  });

  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name || b.getAttribute("aria-label") === name);

  it("is_a_polite_live_region_that_is_empty_without_toasts", () => {
    expect(container.querySelector("section")?.getAttribute("aria-live")).toBe("polite");
    expect(container.querySelectorAll("[role=status], [role=alert]")).toHaveLength(0);
  });

  it("shows_info_as_status_and_errors_as_alert", () => {
    act(() => {
      toast.info("Gespeichert");
      toast.error("Die Datei fehlt.");
    });
    expect(container.querySelector("[role=status]")?.textContent).toContain("Gespeichert");
    expect(container.querySelector("[role=alert]")?.textContent).toContain("Die Datei fehlt.");
  });

  it("an_info_disappears_after_five_seconds_but_an_error_stays", () => {
    act(() => {
      toast.info("Gespeichert");
      toast.error("Die Datei fehlt.");
    });
    act(() => {
      vi.advanceTimersByTime(INFO_MS);
    });
    expect(container.querySelector("[role=status]")).toBeNull();
    expect(container.querySelector("[role=alert]")).not.toBeNull();
  });

  it("progress_replaces_instead_of_stacking", () => {
    act(() => toast.progress("load", "Lade 1 von 3"));
    act(() => toast.progress("load", "Lade 2 von 3"));
    const items = container.querySelectorAll("[role=status]");
    expect(items).toHaveLength(1);
    expect(items[0]?.textContent).toContain("Lade 2 von 3");
    act(() => toast.progress("load", null));
    expect(container.querySelectorAll("[role=status]")).toHaveLength(0);
  });

  it("shows_at_most_four_toasts_with_the_newest_last", () => {
    act(() => {
      for (let i = 1; i <= 6; i++) toast.error(`Fehler ${i}`);
    });
    const texts = [...container.querySelectorAll("[role=alert]")].map((el) => el.textContent ?? "");
    expect(texts).toHaveLength(4);
    expect(texts[0]).toContain("Fehler 3");
    expect(texts[3]).toContain("Fehler 6");
  });

  it("hovering_a_toast_holds_the_clock", () => {
    act(() => {
      toast.info("Gespeichert");
    });
    const item = container.querySelector("[role=status]")!;
    // React derives mouseenter/mouseleave from mouseover/mouseout.
    act(() => {
      item.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body }));
    });
    act(() => {
      vi.advanceTimersByTime(INFO_MS * 2);
    });
    expect(container.querySelector("[role=status]")).not.toBeNull();
    act(() => {
      item.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: document.body }));
    });
    act(() => {
      vi.advanceTimersByTime(INFO_MS);
    });
    expect(container.querySelector("[role=status]")).toBeNull();
  });

  it("action_buttons_run_and_close_the_toast", () => {
    const onClick = vi.fn();
    act(() => {
      toast.error("1 Fehler in Scripts, Build abgebrochen", { actions: [{ label: "Probleme zeigen", onClick }] });
    });
    act(() => button("Probleme zeigen")!.click());
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(container.querySelector("[role=alert]")).toBeNull();
  });

  it("the_close_button_dismisses_the_toast", () => {
    act(() => {
      toast.error("Die Datei fehlt.");
    });
    act(() => button("Meldung schließen")!.click());
    expect(container.querySelector("[role=alert]")).toBeNull();
  });
});
