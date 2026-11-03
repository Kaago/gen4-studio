// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DataRowDto } from "../../api";
import { trainerRow } from "../../data/trainer/trainerTesting";
import { TrainerField } from "./TrainerField";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TrainerField", () => {
  let container: HTMLElement;
  let root: Root;
  const onPick = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onPick.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];

  it("the_picker_leaves_deleted_trainers_out", async () => {
    // Jens and Mia live, 930 is a tombstone („die Sperre verbietet neue Verweise“).
    const trainers: DataRowDto[] = [
      trainerRow(12, "Jens", 6, "Wanderer", 12, []),
      trainerRow(13, "Mia", 7, "Göre", 9, []),
      { ...trainerRow(930, "Neu", 6, "Wanderer", 5, []), deleted: true },
    ];
    await act(async () => root.render(<TrainerField script={0} family="Pt" trainers={trainers} goTo={() => {}} onPick={onPick} />));
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Trainer ändern…"]')!.click());
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining("Wanderer Jens"), expect.stringContaining("Göre Mia")]);
    expect(options().some((o) => o.textContent?.includes("0930"))).toBe(false);
  });
});
