// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AdoptProgressDto, AdoptReportDto } from "../../api";
import { AdoptBox } from "./AdoptBox";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

describe("AdoptBox", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onClose.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const click = (el: Element | undefined) => act(() => void (el as HTMLElement).click());
  const show = (adopt: Parameters<typeof AdoptBox>[0]["adopt"], fileCount = 7) => act(() => root.render(<AdoptBox fileCount={fileCount} adopt={adopt} onClose={onClose} />));

  it("explains_what_adoption_does_and_offers_the_button", () => {
    show(vi.fn());
    expect(container.textContent).toContain("Scripts übernehmen");
    expect(container.textContent).toContain("rotom.toml");
    expect(container.textContent).toContain("alle 7 Scriptdateien");
    expect(container.textContent).toContain("Scripteditor");
    expect(button("Übernehmen")).toBeDefined();
    expect(container.querySelector('[role="progressbar"]')).toBeNull();
  });

  it("is_an_info_notice_titled_with_what_it_does", () => {
    show(vi.fn());
    const notice = container.querySelector('[role="status"]');
    expect(notice).not.toBeNull();
    expect(notice?.querySelector("p")?.textContent).toBe("Scripts übernehmen");
    expect(notice?.contains(button("Übernehmen")!)).toBe(true);
  });

  it("shows_the_progress_while_it_runs_and_the_report_after", async () => {
    const run = deferred<AdoptReportDto>();
    let report!: (p: AdoptProgressDto) => void;
    const adopt = vi.fn((onProgress: (p: AdoptProgressDto) => void) => ((report = onProgress), run.promise));
    show(adopt);
    click(button("Übernehmen"));
    expect(adopt).toHaveBeenCalledTimes(1);
    expect(button("Übernehmen")).toBeUndefined();
    const bar = () => container.querySelector<HTMLElement>('[role="progressbar"]');
    expect(bar()).not.toBeNull();
    expect(bar()?.getAttribute("aria-valuenow")).toBe("0");
    act(() => report({ done: 3, total: 7 }));
    expect([bar()?.getAttribute("aria-valuenow"), bar()?.getAttribute("aria-valuemax")]).toEqual(["3", "7"]);
    expect(bar()?.getAttribute("aria-label")).toBe("Fortschritt der Übernahme");
    // The fill is the one runtime value of the box.
    expect((bar()?.firstElementChild as HTMLElement).style.width).toBe(`${(3 / 7) * 100}%`);
    expect(container.textContent).toContain("3 / 7 Scriptdateien");
    await act(async () => run.resolve({ decompiled: 5, skipped_empty: 1, failed: [[6, "unbekannter Befehl"]] }));
    expect(bar()).toBeNull();
    expect(container.textContent).toContain("5 Scriptdateien dekompiliert, 1 leere übersprungen, 1 fehlgeschlagen.");
    expect(container.textContent).toContain("Scriptdatei 0006: unbekannter Befehl");
    // Progress that arrives after the end does not bring the bar back.
    act(() => report({ done: 7, total: 7 }));
    expect(bar()).toBeNull();
    click(button("Weiter"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows_the_error_and_lets_the_user_try_again", async () => {
    const adopt = vi.fn().mockRejectedValueOnce({ code: "error", message: "rotom.toml nicht schreibbar", asset: null }).mockResolvedValue({ decompiled: 1, skipped_empty: 0, failed: [] });
    show(adopt);
    click(button("Übernehmen"));
    await act(async () => {});
    expect(container.querySelector('[role="alert"]')?.textContent).toBe("rotom.toml nicht schreibbar");
    click(button("Übernehmen"));
    await act(async () => {});
    expect(container.textContent).toContain("1 Scriptdateien dekompiliert");
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });
});
