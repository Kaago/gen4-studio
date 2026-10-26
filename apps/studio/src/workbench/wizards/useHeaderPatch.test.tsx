// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyHeaderPatch, dirty, headerPatchStatus } from "../../api";
import type { HeaderPatchDto } from "../../api";
import { useHeaderPatch } from "./useHeaderPatch";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), headerPatchStatus: vi.fn(), applyHeaderPatch: vi.fn(), dirty: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const state = (over: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders", ...over });

describe("useHeaderPatch", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: ReturnType<typeof useHeaderPatch>;
  const applied = vi.fn(async () => {});

  function Probe() {
    shown = useHeaderPatch(applied);
    return null;
  }

  beforeEach(() => {
    applied.mockClear();
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue(state());
    vi.mocked(applyHeaderPatch).mockReset().mockResolvedValue({ ...state({ applied: true }), snapshot: null });
    vi.mocked(dirty).mockReset().mockResolvedValue([]);
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("applies_with_an_unsaved_table", async () => {
    // The kernel keeps unsaved header edits across the patch: nothing asks to save first.
    vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable", index: 0 }]);
    await act(async () => root.render(<Probe />));
    expect(shown.patch?.applied).toBe(false);
    await act(async () => shown.apply());
    expect(applyHeaderPatch).toHaveBeenCalledOnce();
    expect(applied).toHaveBeenCalledOnce();
    expect(shown.error).toBeNull();
    expect(shown.patch?.applied).toBe(true);
  });

  it("shows_the_backends_refusal", async () => {
    vi.mocked(applyHeaderPatch).mockRejectedValue({ code: "stale", message: "the header table changed on disk", asset: null });
    await act(async () => root.render(<Probe />));
    await act(async () => shown.apply());
    expect(applied).not.toHaveBeenCalled();
    expect(shown.error).toContain("the header table changed on disk");
    expect(shown.patch?.applied).toBe(false);
  });
});
