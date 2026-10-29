// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { gitStatusOf, noRepo } from "../../git/gitTesting";
import { GitStatusItem } from "./GitStatusItem";
import type { GitStatusItemProps } from "./GitStatusItem";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("GitStatusItem", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpen = vi.fn();

  const show = (props: Partial<GitStatusItemProps>) => act(async () => root.render(<GitStatusItem status={null} onOpen={onOpen} {...props} />));
  const item = () => container.querySelector<HTMLButtonElement>("[data-git-status]");
  /** The tooltip the item shows on hover. */
  async function tooltip(): Promise<string | null> {
    vi.useFakeTimers();
    try {
      await act(async () => {
        const b = item()!;
        b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        b.dispatchEvent(new MouseEvent("mouseenter"));
        b.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(500);
      });
      return document.querySelector("[role=tooltip]")?.textContent ?? null;
    } finally {
      vi.useRealTimers();
    }
  }

  beforeEach(() => {
    onOpen.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("hidden_without_a_repo", async () => {
    await show({ status: null });
    expect(container.innerHTML).toBe("");
    await show({ status: noRepo() });
    expect(container.innerHTML).toBe("");
  });

  it("shows_branch_and_count_and_opens_technik", async () => {
    await show({ status: gitStatusOf({ changes: 3 }) });
    expect(item()?.textContent).toBe("main · 3 Änderungen");
    // The git symbol, a quiet small button.
    expect(item()?.querySelector("svg")).not.toBeNull();
    expect(item()?.dataset.kind).toBe("quiet");
    expect(await tooltip()).toBe("Git: 3 geänderte Dateien seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.");
    await act(async () => item()!.click());
    expect(onOpen).toHaveBeenCalledOnce();
    await show({ status: gitStatusOf({ changes: 0 }) });
    expect(item()?.textContent).toBe("main · keine Änderung");
  });

  it("unborn_and_detached_read_as_such", async () => {
    await show({ status: gitStatusOf({ head: null, changes: 12 }) });
    expect(item()?.textContent).toBe("main · noch kein Commit");
    expect(await tooltip()).toBe("Git: noch kein Commit.");
    await show({ status: gitStatusOf({ branch: null, detached: true, changes: 1 }) });
    expect(item()?.textContent).toBe("4c1d2e3 (losgelöst) · 1 Änderung");
  });

  it("an_unreadable_status_names_its_reason", async () => {
    await show({ status: null, error: "Das Repository ist beschädigt." });
    expect(item()?.textContent).toBe("Git: nicht lesbar");
    expect(await tooltip()).toBe("Das Repository ist beschädigt.");
  });
});
