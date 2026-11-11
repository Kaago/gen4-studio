// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ago } from "../ui/frame/timeAgo";
import { UpdatesSettings } from "./UpdatesSettings";
import type { UpdatesSettingsProps } from "./UpdatesSettings";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("UpdatesSettings", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onAuto: vi.fn(), onCheckNow: vi.fn() };
  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  const render = (props: Partial<UpdatesSettingsProps> = {}) =>
    act(() => root.render(<UpdatesSettings version="1.0.1" auto={true} lastCheck={null} checking={false} {...handlers} {...props} />));
  const heading = () => container.querySelector("h2");
  const toggle = () => container.querySelector<HTMLElement>("[role=switch]");
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);

  it("is_a_column_named_by_its_heading_with_the_version", () => {
    render();
    expect(heading()?.textContent).toBe("Updates");
    const section = container.querySelector("section")!;
    expect(document.getElementById(section.getAttribute("aria-labelledby") ?? "")).toBe(heading());
    expect(container.textContent).toContain("Version 1.0.1");
    expect(container.textContent).toContain("Höchstens einmal am Tag beim Start, auf GitHub. Über Projekte wird nichts gesendet.");
  });

  it("the_switch_follows_auto_and_a_click_reports_the_opposite", () => {
    render({ auto: true });
    expect(container.textContent).toContain("Automatisch nach Updates suchen");
    expect(toggle()?.getAttribute("aria-checked")).toBe("true");
    act(() => toggle()!.click());
    expect(handlers.onAuto).toHaveBeenCalledExactlyOnceWith(false);
    handlers.onAuto.mockClear();
    render({ auto: false });
    expect(toggle()?.getAttribute("aria-checked")).toBe("false");
    act(() => toggle()!.click());
    expect(handlers.onAuto).toHaveBeenCalledExactlyOnceWith(true);
  });

  it("check_now_calls_onCheckNow_and_is_locked_while_checking", () => {
    render();
    act(() => button("Jetzt suchen")!.click());
    expect(handlers.onCheckNow).toHaveBeenCalledOnce();
    handlers.onCheckNow.mockClear();
    render({ checking: true });
    expect(button("Jetzt suchen")!.disabled).toBe(true);
    act(() => button("Jetzt suchen")!.click());
    expect(handlers.onCheckNow).not.toHaveBeenCalled();
  });

  it("says_not_yet_checked_without_a_last_check", () => {
    render({ lastCheck: null });
    expect(container.textContent).toContain("Noch nicht gesucht");
    expect(container.textContent).not.toContain("Zuletzt gesucht");
  });

  it("says_when_it_last_checked_with_the_age", () => {
    const now = Date.parse("2026-11-12T12:00:00Z");
    vi.useFakeTimers({ now });
    const from = now - 4 * 60_000;
    render({ lastCheck: new Date(from).toISOString() });
    expect(ago(from, now)).toBe("vor 4 Minuten");
    expect(container.textContent).toContain(`Zuletzt gesucht ${ago(from, now)}`);
    expect(container.textContent).not.toContain("Noch nicht gesucht");
  });

  it("a_last_check_that_is_no_date_counts_as_not_yet_checked", () => {
    render({ lastCheck: "gestern" });
    expect(container.textContent).toContain("Noch nicht gesucht");
  });
});
