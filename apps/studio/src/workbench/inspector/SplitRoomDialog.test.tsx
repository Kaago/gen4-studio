// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { splitRoom } from "../../api";
import { sharedText } from "../sharedRooms";
import { SplitRoomDialog } from "./SplitRoomDialog";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  splitRoom: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("SplitRoomDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onSplit = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(splitRoom).mockReset().mockResolvedValue({ matrix: 700, maps: [601, 602] });
    for (const fn of [onClose, onSplit, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const button = (text: string) => [...(dialog()?.querySelectorAll("button") ?? [])].find((b) => b.textContent === text);
  const show = async (maps: number | null = 2, open = true) => {
    await act(async () => root.render(<SplitRoomDialog open={open} header={412} name="Haus 1" maps={maps} onClose={onClose} onSplit={onSplit} onError={onError} />));
    await flush();
  };

  it("says_what_splitting_does_with_the_not_undoable_badge", async () => {
    await show();
    expect(dialog()?.textContent).toContain("Raum abspalten?");
    expect(dialog()?.textContent).toContain("Nicht rückgängig");
    expect(dialog()?.textContent).toContain(
      "Der Raum „Haus 1“ bekommt eigene Kopien seiner 2 Karten. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopien lassen sich nicht zurücknehmen.",
    );
    expect([button("Abbrechen"), button("Abspalten")].every((b) => b !== undefined)).toBe(true);
    // One map: the sentence speaks of one copy.
    await show(1);
    expect(dialog()?.textContent).toContain(
      "Der Raum „Haus 1“ bekommt eine eigene Kopie seiner Karte. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopie lässt sich nicht zurücknehmen.",
    );
    // Not known: the sentence names no number.
    await show(null);
    expect(dialog()?.textContent).toContain(
      "Der Raum „Haus 1“ bekommt eigene Kopien seiner Karten. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopien lassen sich nicht zurücknehmen.",
    );
    // Closed: nothing shows.
    await show(2, false);
    expect(dialog()).toBeNull();
  });

  it("splits_and_reports", async () => {
    await show();
    await act(async () => button("Abspalten")!.click());
    await flush();
    expect(splitRoom).toHaveBeenCalledWith(412, "Haus 1");
    expect(onSplit).toHaveBeenCalledWith({ matrix: 700, maps: [601, 602] });
    expect(onClose).toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("keeps_the_dialog_open_on_a_refusal_and_cancels_without_a_call", async () => {
    vi.mocked(splitRoom).mockReset().mockRejectedValue("Dieser Raum teilt seine Karten mit niemandem.");
    await show();
    await act(async () => button("Abspalten")!.click());
    await flush();
    expect(onError).toHaveBeenCalledWith("Dieser Raum teilt seine Karten mit niemandem.");
    expect(onSplit).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    vi.mocked(splitRoom).mockClear();
    await act(async () => button("Abbrechen")!.click());
    expect(onClose).toHaveBeenCalled();
    expect(splitRoom).not.toHaveBeenCalled();
  });

  it("reports_a_failing_follow_up_instead_of_losing_it", async () => {
    onSplit.mockRejectedValue("Das Projekt ließ sich nicht neu lesen.");
    await show();
    await act(async () => button("Abspalten")!.click());
    await flush();
    expect(splitRoom).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalled();
    expect(onError).toHaveBeenCalledWith("Das Projekt ließ sich nicht neu lesen.");
  });

  it("names_the_number_of_rooms_shared_with", () => {
    expect(sharedText(17)).toBe("Geteilt mit 17 Räumen");
    expect(sharedText(1)).toBe("Geteilt mit 1 Raum");
  });
});
