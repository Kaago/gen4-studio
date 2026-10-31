// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { returnFocus, useWizards } from "./useWizards";
import type { Wizards } from "./useWizards";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useWizards", () => {
  it("rereads_after_a_split_without_closing_an_open_wizard", async () => {
    const onCreated = vi.fn(async () => {});
    let wizards: Wizards | null = null;
    function Host() {
      wizards = useWizards({ onCreated });
      return null;
    }
    const container = document.createElement("div");
    const root = createRoot(container);
    act(() => root.render(<Host />));
    act(() => wizards!.open({ kind: "area" }));
    const made = { matrix: 700, header: 412, announcement: "Raum „Haus 1“ abgespalten" };
    await act(async () => wizards!.reread(made));
    expect(onCreated).toHaveBeenCalledWith(made);
    expect(wizards!.request).toEqual({ kind: "area" });
    // „Zeigen“ of a wizard leaves it.
    await act(async () => wizards!.created(made));
    expect(wizards!.request).toBeNull();
    act(() => root.unmount());
  });
  it("leaving_without_zeigen_goes_back_once_and_closing_nothing_does_not", async () => {
    const onLeave = vi.fn();
    let wizards: Wizards | null = null;
    function Host() {
      wizards = useWizards({ onCreated: async () => {}, onLeave });
      return null;
    }
    const container = document.createElement("div");
    const root = createRoot(container);
    act(() => root.render(<Host />));
    // Nothing open (another project opened): nothing to go back to.
    act(() => wizards!.close());
    expect(onLeave).not.toHaveBeenCalled();
    act(() => wizards!.open({ kind: "publish" }));
    act(() => wizards!.close(true));
    expect(onLeave).toHaveBeenCalledOnce();
    // „Zeigen“ shows what it made in the world: no way back.
    act(() => wizards!.open({ kind: "area" }));
    await act(async () => wizards!.created({ matrix: 1, header: 2, announcement: "" }));
    expect(onLeave).toHaveBeenCalledOnce();
    act(() => root.unmount());
  });

  it("return_focus_prefers_the_opener_then_its_twin_in_the_shown_area_then_the_areas_first_control", () => {
    document.body.innerHTML = `
      <main hidden><button type="button">Veröffentlichen…</button></main>
      <main><button type="button" id="first">Prüfen</button><button type="button" id="twin">Veröffentlichen…</button></main>
      <button type="button" id="opener">Veröffentlichen…</button>`;
    const byId = (id: string) => document.getElementById(id)!;
    // The opener stands: it gets the focus.
    returnFocus(byId("opener"));
    expect(document.activeElement).toBe(byId("opener"));
    // Something has the focus already (the frame gave it back): it stays.
    byId("first").focus();
    returnFocus(byId("opener"));
    expect(document.activeElement).toBe(byId("first"));
    // The opener is gone (its area was rebuilt): the control of the same name in the shown area, not the hidden one's.
    byId("first").blur();
    const gone = byId("opener");
    gone.remove();
    returnFocus(gone);
    expect(document.activeElement).toBe(byId("twin"));
    // Nothing of that name (the palette's field): the shown area's first control.
    byId("twin").blur();
    returnFocus(null);
    expect(document.activeElement).toBe(byId("first"));
    document.body.innerHTML = "";
  });
});
