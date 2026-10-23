// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PickerItem } from "../primitives";
import { pressKey, typeInSearch } from "../primitives/testUtils";
import { Palette } from "./Palette";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Entry = PickerItem & { run: string };

const commands: Entry[] = Array.from({ length: 10 }, (_, i) => ({ id: `cmd-${i}`, label: `Befehl ${i}`, group: "Befehle", icon: "plus", run: `c${i}` }));
const entries: Entry[] = [
  { id: "cmd-save", label: "Speichern", group: "Befehle", icon: "save", shortcut: "⌘S", run: "save" },
  ...commands,
  { id: "place-Zweiblattdorf", label: "Zweiblattdorf", group: "Orte", area: "welt", run: "place" },
  { id: "header-412", label: "Zweiblattdorf", tech: "0412", description: "T01R0101", group: "Header", area: "welt", run: "header" },
  ...Array.from({ length: 12 }, (_, i): Entry => ({ id: `header-${500 + i}`, label: `Route ${i}`, tech: String(500 + i).padStart(4, "0"), group: "Header", area: "welt", run: "h" })),
  { id: "archive-412", label: "Zweiblattdorf", tech: "0412", group: "Textarchive", area: "texte", run: "archive" },
];

describe("Palette", () => {
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

  const input = () => document.querySelector<HTMLInputElement>("input[role=combobox]")!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const groups = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=group]")];
  const tick = () =>
    act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

  /** The palette as App holds it: open state of its own, `onRun` with the entry picked. */
  function mount(onRun: (entry: Entry) => void) {
    const changes: boolean[] = [];
    function Host() {
      const [open, set] = useState(true);
      return (
        <Palette
          open={open}
          onOpenChange={(next) => {
            changes.push(next);
            set(next);
          }}
          items={entries}
          onRun={onRun}
        />
      );
    }
    act(() => root.render(<Host />));
    return { changes };
  }

  it("shows_only_the_commands_at_most_eight_until_something_is_typed", () => {
    mount(vi.fn());
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Befehlspalette");
    expect(groups()).toHaveLength(1);
    expect(groups()[0]!.textContent).toContain("Befehle");
    expect(options()).toHaveLength(8);
    expect(options().every((o) => !o.textContent?.includes("Zweiblattdorf"))).toBe(true);
    // The count is taken before the cut.
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("11 Treffer");
  });

  it("finds_0412_as_header_and_text_archive_in_two_groups_with_the_id_grey", () => {
    mount(vi.fn());
    typeInSearch(input(), "0412");
    expect(groups()).toHaveLength(2);
    expect(groups()[0]!.textContent).toContain("Header");
    expect(groups()[1]!.textContent).toContain("Textarchive");
    expect(options()).toHaveLength(2);
    expect(options().every((o) => o.textContent?.includes("0412"))).toBe(true);
  });

  it("lists_at_most_eight_per_group_while_searching", () => {
    mount(vi.fn());
    typeInSearch(input(), "route");
    expect(options()).toHaveLength(8);
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("12 Treffer");
  });

  it("runs_the_entry_on_enter_after_the_palette_closed", async () => {
    const order: string[] = [];
    const onSave = vi.fn(() => order.push("save"));
    const { changes } = mount((entry) => {
      order.push(`closed ${document.querySelector("[role=dialog]") === null || changes.includes(false)}`);
      if (entry.run === "save") onSave();
    });
    typeInSearch(input(), "speich");
    pressKey(input(), "Enter");
    expect(changes).toContain(false);
    expect(onSave).not.toHaveBeenCalled();
    await tick();
    expect(onSave).toHaveBeenCalledOnce();
    expect(order).toEqual(["closed true", "save"]);
  });

  it("closes_on_escape_without_running_anything", async () => {
    const onRun = vi.fn();
    const { changes } = mount(onRun);
    typeInSearch(input(), "speich");
    pressKey(input(), "Escape");
    await tick();
    expect(changes).toEqual([false]);
    expect(onRun).not.toHaveBeenCalled();
  });
});
