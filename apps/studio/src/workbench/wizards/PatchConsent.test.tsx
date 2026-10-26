// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderPatchDto } from "../../api";
import { visibleWords } from "../../ui/visibleText";
import { PatchConsent, patchReady } from "./PatchConsent";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const CONSENT = "Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)";
const patch = (change: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders", ...change });

describe("PatchConsent", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onChange.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (state: HeaderPatchDto | null, checked = false, error: string | null = null) => act(async () => root.render(<PatchConsent patch={state} error={error} checked={checked} onChange={onChange} />));

  it("says_the_one_time_consent_with_the_not_undoable_badge", async () => {
    await show(patch());
    expect(container.textContent).toContain("Nicht rückgängig");
    expect(container.textContent).toContain(CONSENT);
    const box = container.querySelector<HTMLElement>("[role=checkbox]")!;
    expect(box.getAttribute("aria-checked")).toBe("false");
    await act(async () => box.click());
    expect(onChange).toHaveBeenCalledWith(true);
    expect(patchReady(patch(), false)).toBe(false);
    expect(patchReady(patch(), true)).toBe(true);
    // With the patch in place there is nothing to agree to.
    await show(patch({ applied: true }));
    expect(container.textContent).toBe("");
    expect(patchReady(patch({ applied: true }), false)).toBe(true);
    expect(patchReady(null, true)).toBe(false);
  });

  it("blocks_on_a_broken_patch", async () => {
    await show(patch({ applied: true, repairable: true }), true);
    expect(container.textContent).toBe("Die Anpassung des Spielcodes für neue Orte ist fehlerhaft. Reparieren in Technik › Patches.");
    expect(container.querySelector("[role=checkbox]")).toBeNull();
    expect(patchReady(patch({ applied: true, repairable: true }), true)).toBe(false);
    // A game without the patch (Diamond/Pearl) cannot create here at all.
    await show(patch({ supported: false, narc: null }));
    expect(container.textContent).toContain("Neue Gebiete und Räume legt Bauen nur für Platin und HeartGold/SoulSilver an. In Tüfteln lässt sich ein vorhandenes Gebiet umstellen.");
    expect(container.querySelector("[role=checkbox]")).toBeNull();
    expect(patchReady(patch({ supported: false }), true)).toBe(false);
  });

  it("says_while_the_state_is_read_and_why_it_could_not_be", async () => {
    await show(null);
    expect(container.textContent).toBe("Prüfe den Spielcode …");
    await show(null, false, "unknown game code XXXX");
    expect(container.querySelector("[role=alert]")).not.toBeNull();
    expect(container.textContent).toBe("unknown game code XXXX");
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    for (const state of [null, patch(), patch({ applied: true, repairable: true }), patch({ supported: false, narc: null })]) {
      await show(state);
      expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    }
  });
});
