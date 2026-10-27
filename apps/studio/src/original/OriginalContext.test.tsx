// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { planReset, resetToOriginal } from "../api";
import type { AssetId, OriginalStatus, ResetResult } from "../api";
import { headerFamilyOf, OriginalProvider, useReset } from "./OriginalContext";
import type { ResetApi } from "./OriginalContext";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  planReset: vi.fn(),
  resetToOriginal: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ready: OriginalStatus = { state: "ready", label: "Platin DE (Testprojekt)", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", counts: { trainer: 10 } };
const result: ResetResult = { changed: [{ kind: "Trainer", index: 2 }], recorded: true, ms: 9 };

describe("OriginalProvider", () => {
  let container: HTMLElement;
  let root: Root;
  let api: ResetApi;
  const onReset = vi.fn();

  function Probe() {
    api = useReset();
    return null;
  }
  const show = (dirty: readonly AssetId[] = []) =>
    act(async () =>
      root.render(
        <OriginalProvider onReset={onReset} status={ready} family="Pt" dirty={dirty}>
          <Probe />
        </OriginalProvider>,
      ),
    );
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const openTitles = () => [...document.querySelectorAll("[role=dialog]")].filter((d) => !d.hasAttribute("data-closed")).map((d) => d.querySelector("h2")?.textContent);

  beforeEach(() => {
    onReset.mockReset();
    vi.mocked(planReset)
      .mockReset()
      .mockResolvedValue({ target: { kind: "trainer", index: 2 }, label: "Trainer 0002", changes: [{ label: "Trainer 0002", before: "a", after: "b" }], fields: [], writes_files: false, same: false });
    vi.mocked(resetToOriginal).mockReset().mockResolvedValue(result);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shares_the_status_and_opens_the_dialog_for_a_target", async () => {
    await show([{ kind: "Trainer", index: 2 }]);
    expect(api.status).toEqual(ready);
    expect(dialog()).toBeNull();
    await act(async () => api.open({ kind: "trainer", index: 2 }));
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Trainer 0002 auf Original zurücksetzen?");
    // The trainer has unsaved edits: the dialog says ⌘Z brings them back.
    expect(dialog()?.textContent).toContain("Auch ungespeicherte Änderungen werden ersetzt; ⌘Z stellt sie wieder her.");
    const reset = [...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Zurücksetzen")!;
    await act(async () => reset.click());
    expect(onReset).toHaveBeenCalledWith(result, "Trainer 0002");
    expect(document.querySelector('[role=dialog]:not([data-closed])')).toBeNull();
  });

  it("a_slow_reset_closes_only_its_own_dialog", async () => {
    await show();
    await act(async () => api.open({ kind: "trainer", index: 2 }));
    let finish: (r: ResetResult) => void = () => {};
    vi.mocked(resetToOriginal).mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
    const reset = [...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Zurücksetzen")!;
    await act(async () => reset.click());
    // Meanwhile another target is opened (a second place asked for it): the first reset finishing must not close it.
    vi.mocked(planReset).mockResolvedValueOnce({ target: { kind: "map", index: 1 }, label: "Karte 0001", changes: [], fields: [], writes_files: false, same: true });
    await act(async () => api.open({ kind: "map", index: 1 }));
    expect(openTitles()).toEqual(["Karte 0001 auf Original zurücksetzen?"]);
    // Closed and another one opened: still no dialog the first reset may close.
    const close = [...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Schließen")!;
    await act(async () => close.click());
    vi.mocked(planReset).mockResolvedValueOnce({ target: { kind: "map", index: 2 }, label: "Karte 0002", changes: [], fields: [], writes_files: false, same: true });
    await act(async () => api.open({ kind: "map", index: 2 }));
    expect(openTitles()).toEqual(["Karte 0002 auf Original zurücksetzen?"]);
    await act(async () => finish(result));
    expect(openTitles()).toEqual(["Karte 0002 auf Original zurücksetzen?"]);
    // The reset did go in: it is reported all the same.
    expect(onReset).toHaveBeenCalledWith(result, "Trainer 0002");
  });

  it("a_clean_target_has_no_unsaved_notice", async () => {
    await show([{ kind: "Trainer", index: 3 }]);
    await act(async () => api.open({ kind: "trainer", index: 2 }));
    expect(dialog()?.textContent).not.toContain("ungespeicherte");
  });

  it("maps_the_game_family_to_the_header_family", () => {
    expect(["Dp", "Pt", "Hgss", "Xy"].map(headerFamilyOf)).toEqual(["DP", "Pt", "HGSS", null]);
  });

  it("without_a_provider_open_does_nothing", async () => {
    await act(async () => root.render(<Probe />));
    expect(api.status).toBeNull();
    api.open({ kind: "map", index: 0 });
    expect(dialog()).toBeNull();
  });
});
