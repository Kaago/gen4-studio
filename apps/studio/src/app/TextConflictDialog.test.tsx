// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pressKey } from "../ui/primitives/testUtils";
import { TextConflictDialog } from "./TextConflictDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TextConflictDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onResolve = vi.fn();
  const onCancel = vi.fn();

  const show = (archives: number[]) => act(() => root.render(<TextConflictDialog archives={archives} onResolve={onResolve} onCancel={onCancel} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);
  const group = (label: string) => dialog()?.querySelector<HTMLElement>(`[role=group][aria-label="${label}"]`);
  const pressed = (label: string) => [...(group(label)?.querySelectorAll("button") ?? [])].filter((b) => b.getAttribute("aria-pressed") === "true").map((b) => b.textContent);
  const option = (label: string, text: string) => [...(group(label)?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);

  beforeEach(() => {
    onResolve.mockReset();
    onCancel.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("asks_per_archive_and_sends_the_choices", () => {
    show([12, 433]);
    expect(dialog()?.hasAttribute("data-text-conflict")).toBe(true);
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Texte auf zwei Seiten geändert");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      "Diese Textarchive wurden als Text (JSON) und als Spieldatei geändert, seit das Studio sie zuletzt gebaut hat, etwa in DSPRE oder durch einen Git-Wechsel. Wähle je Archiv, welche Fassung gilt; die andere wird überschrieben.",
    );
    const rows = [...(dialog()?.querySelectorAll<HTMLElement>("[data-archive]") ?? [])];
    expect(rows.map((r) => r.dataset.archive)).toEqual(["12", "433"]);
    expect(rows.map((r) => r.querySelector("[data-archive-name]")?.textContent)).toEqual(["Textarchiv 0012", "Textarchiv 0433"]);
    // Each archive starts with the JSON.
    expect(pressed("Fassung von Textarchiv 0012")).toEqual(["Text (JSON)"]);
    expect(pressed("Fassung von Textarchiv 0433")).toEqual(["Text (JSON)"]);
    act(() => option("Fassung von Textarchiv 0433", "Spieldatei")!.click());
    expect(pressed("Fassung von Textarchiv 0433")).toEqual(["Spieldatei"]);
    expect(button("Übernehmen und bauen")?.dataset.kind).toBe("primary");
    act(() => button("Übernehmen und bauen")!.click());
    expect(onResolve.mock.calls).toEqual([
      [
        [
          { archive: 12, keep: "json" },
          { archive: 433, keep: "member" },
        ],
      ],
    ]);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("cancel_builds_nothing", () => {
    show([7]);
    act(() => button("Abbrechen")!.click());
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onResolve).not.toHaveBeenCalled();

    // Escape is a cancel too, and is reported once.
    onCancel.mockReset();
    act(() => root.unmount());
    root = createRoot(container);
    show([7]);
    act(() => void pressKey((document.activeElement as HTMLElement) ?? dialog()!, "Escape"));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onResolve).not.toHaveBeenCalled();
  });

  it("waits_while_the_choice_is_written", () => {
    act(() => root.render(<TextConflictDialog archives={[12]} onResolve={onResolve} onCancel={onCancel} busy />));
    expect(button("Übernehmen und bauen")?.disabled).toBe(true);
    expect(button("Abbrechen")?.disabled).toBe(true);
    expect(option("Fassung von Textarchiv 0012", "Spieldatei")?.disabled).toBe(true);
    expect(dialog()?.querySelector("[role=status]")?.textContent).toBe("Die gewählten Fassungen werden übernommen, danach startet der Build …");
    // Escape does not cancel what is being written.
    pressKey((document.activeElement as HTMLElement) ?? dialog()!, "Escape");
    expect(onCancel).not.toHaveBeenCalled();
    expect(dialog()).not.toBeNull();
  });
});
