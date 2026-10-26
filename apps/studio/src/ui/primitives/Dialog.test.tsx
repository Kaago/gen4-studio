// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Dialog } from "./Dialog";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Dialog", () => {
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

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");

  it("open_renders_a_modal_dialog_named_by_its_title", () => {
    act(() =>
      root.render(
        <Dialog open onOpenChange={() => {}} title="Tür wirklich löschen?" description="Das lässt sich nicht zurücknehmen.">
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    const el = dialog()!;
    expect(el).not.toBeNull();
    expect(el.getAttribute("aria-modal")).toBe("true");
    const titleId = el.getAttribute("aria-labelledby")!;
    expect(document.getElementById(titleId)?.textContent).toBe("Tür wirklich löschen?");
    expect(el.textContent).toContain("Das lässt sich nicht zurücknehmen.");
    expect(el.textContent).toContain("Inhalt");
  });

  it("palette_variant_has_no_close_button_and_keeps_the_title_as_its_name", () => {
    act(() =>
      root.render(
        <Dialog open onOpenChange={() => {}} title="Befehlspalette" variant="palette">
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    const el = dialog()!;
    expect(el.querySelector("[aria-label=Schließen]")).toBeNull();
    expect(document.getElementById(el.getAttribute("aria-labelledby")!)?.textContent).toBe("Befehlspalette");
  });

  it("closed_renders_nothing", () => {
    act(() =>
      root.render(
        <Dialog open={false} onOpenChange={() => {}} title="Frage">
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    expect(dialog()).toBeNull();
  });

  it("data_attributes_go_onto_the_popup", () => {
    act(() => root.render(<Dialog open onOpenChange={() => {}} title="Frage" data={{ "data-probe": "ja" }} />));
    expect(dialog()?.getAttribute("data-probe")).toBe("ja");
  });

  it("escape_asks_the_owner_to_close", () => {
    const onOpenChange = vi.fn();
    act(() =>
      root.render(
        <Dialog open onOpenChange={onOpenChange} title="Frage">
          <button type="button">Eins</button>
        </Dialog>,
      ),
    );
    pressKey((document.activeElement as HTMLElement) ?? dialog()!, "Escape");
    expect(onOpenChange.mock.calls.map((c) => c[0])).toEqual([false]);
  });

  it("escape_that_a_field_used_does_not_ask_the_owner_to_close", () => {
    const onOpenChange = vi.fn();
    act(() =>
      root.render(
        <Dialog open onOpenChange={onOpenChange} title="Frage">
          <input
            aria-label="Feld"
            onKeyDown={(e) => {
              if (e.key === "Escape") e.preventDefault();
            }}
          />
        </Dialog>,
      ),
    );
    pressKey(dialog()!.querySelector("input")!, "Escape");
    expect(onOpenChange).not.toHaveBeenCalled();
    pressKey(dialog()!.querySelector("input")!, "Enter");
    pressKey(dialog()!, "Escape");
    expect(onOpenChange.mock.calls.map((c) => c[0])).toEqual([false]);
  });

  it("the_close_button_asks_the_owner_to_close", () => {
    const onOpenChange = vi.fn();
    act(() =>
      root.render(
        <Dialog open onOpenChange={onOpenChange} title="Frage">
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    act(() => dialog()!.querySelector<HTMLElement>("button[aria-label='Schließen']")!.click());
    expect(onOpenChange.mock.calls.map((c) => c[0])).toEqual([false]);
  });

  it("actions_are_buttons_in_the_given_order", () => {
    const remove = vi.fn();
    const keep = vi.fn();
    act(() =>
      root.render(
        <Dialog
          open
          onOpenChange={() => {}}
          title="Tür wirklich löschen?"
          actions={[
            { label: "Behalten", onClick: keep },
            { label: "Tür löschen", kind: "danger", onClick: remove },
          ]}
        >
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    const names = [...dialog()!.querySelectorAll("button")].map((b) => b.textContent || b.getAttribute("aria-label"));
    expect(names).toEqual(["Behalten", "Tür löschen", "Schließen"]);
    act(() => [...dialog()!.querySelectorAll("button")].find((b) => b.textContent === "Tür löschen")!.click());
    expect(remove).toHaveBeenCalledTimes(1);
    expect(keep).not.toHaveBeenCalled();
  });

  it("focus_moves_into_the_dialog_when_it_opens", async () => {
    act(() =>
      root.render(
        <Dialog open onOpenChange={() => {}} title="Frage" actions={[{ label: "Ja", onClick: () => {} }]}>
          <p>Inhalt</p>
        </Dialog>,
      ),
    );
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    expect(dialog()!.contains(document.activeElement)).toBe(true);
  });
});
