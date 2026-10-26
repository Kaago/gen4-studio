// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { useWizards } from "./useWizards";
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
});
