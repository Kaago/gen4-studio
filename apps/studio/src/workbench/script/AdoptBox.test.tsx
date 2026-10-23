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
    expect(container.querySelector("progress")).toBeNull();
  });

  it("sets_its_own_text_colour_with_its_surface_so_the_dark_theme_stays_readable", () => {
    show(vi.fn());
    const box = container.firstElementChild as HTMLElement;
    expect(box.style.background).toBe("var(--g4-flaeche-blatt)");
    expect(box.style.color).toBe("var(--g4-text-primaer)");
  });

  it("shows_the_progress_while_it_runs_and_the_report_after", async () => {
    const run = deferred<AdoptReportDto>();
    let report!: (p: AdoptProgressDto) => void;
    const adopt = vi.fn((onProgress: (p: AdoptProgressDto) => void) => ((report = onProgress), run.promise));
    show(adopt);
    click(button("Übernehmen"));
    expect(adopt).toHaveBeenCalledTimes(1);
    expect(button("Übernehmen")).toBeUndefined();
    expect(container.querySelector("progress")).not.toBeNull();
    act(() => report({ done: 3, total: 7 }));
    const bar = container.querySelector("progress")!;
    expect([bar.value, bar.max]).toEqual([3, 7]);
    expect(container.textContent).toContain("3 / 7 Scriptdateien");
    await act(async () => run.resolve({ decompiled: 5, skipped_empty: 1, failed: [[6, "unbekannter Befehl"]] }));
    expect(container.querySelector("progress")).toBeNull();
    expect(container.textContent).toContain("5 Scriptdateien dekompiliert, 1 leere übersprungen, 1 fehlgeschlagen.");
    expect(container.textContent).toContain("Scriptdatei 0006: unbekannter Befehl");
    // Progress that arrives after the end does not bring the bar back.
    act(() => report({ done: 7, total: 7 }));
    expect(container.querySelector("progress")).toBeNull();
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
