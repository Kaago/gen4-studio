// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../../api";
import type { EventsDto, HeaderSummary, ObjectDto } from "../../api";
import { FigureField, isTrainerFigure } from "./FigureField";
import { trainersTalk } from "./storyRecipeModel";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn() }));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const town: HeaderSummary = { id: 3, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 8, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };
const object = (index: number, over: Partial<ObjectDto>): ObjectDto => ({ index, local_id: index, graphics_id: 0, movement_type: 0, script: 0, dir: 0, trainer_type: 0, sight: 0, hidden_flag: 0, x: 0, z: 0, y: 0, ...over });
const events: EventsDto = {
  event_file_id: 8,
  locked: null,
  bg: [],
  warps: [],
  coords: [],
  objects: [object(0, { local_id: 1, graphics_id: 97, script: 5 }), object(1, { local_id: 2, graphics_id: 12, script: 3001, trainer_type: 1, sight: 3 })],
};

describe("FigureField", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onChange.mockReset();
    vi.mocked(getEvents).mockReset().mockResolvedValue(events);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];

  it("the_figure_field_lists_the_objects_of_the_header_and_locks_trainers_for_talk", async () => {
    await act(async () => root.render(<FigureField header={3} localId={null} onChange={onChange} headers={[town]} revision={0} lockTrainers onError={vi.fn()} />));
    await flush();
    // The place by its name, the figure not chosen yet.
    expect(container.querySelector("[data-figure-place]")?.textContent).toContain("Zweiblattdorf");
    expect(getEvents).toHaveBeenCalledWith(8);
    const pick = container.querySelector<HTMLButtonElement>('button[aria-label="Figur wählen…"]')!;
    await act(async () => pick.click());
    await flush();
    const [first, trainer] = options();
    expect(first?.textContent).toContain("Figur 1 · Aussehen 97");
    // A trainer carries its badge and cannot be picked for talk or a reward: it speaks through its battle texts.
    expect(trainer?.textContent).toContain("Figur 2 · Aussehen 12");
    expect(trainer?.textContent).toContain("Trainer");
    expect(trainer?.textContent).toContain(trainersTalk());
    expect(trainer?.getAttribute("aria-disabled") ?? trainer?.getAttribute("data-disabled")).not.toBeNull();
    await act(async () => trainer!.click());
    expect(onChange).not.toHaveBeenCalled();
    await act(async () => first!.click());
    await flush();
    expect(onChange).toHaveBeenCalledExactlyOnceWith({ header: 3, local_id: 1 });

    // For an appearance a trainer is a figure like any other.
    await act(async () => root.render(<FigureField header={3} localId={2} onChange={onChange} headers={[town]} revision={0} onError={vi.fn()} />));
    await flush();
    expect(container.querySelector("[data-figure-pick]")?.textContent).toContain("Figur 2 · Aussehen 12");
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Figur wählen…"]')!.click());
    await flush();
    expect(options()[1]?.getAttribute("aria-disabled")).not.toBe("true");
  });

  it("a_trainer_is_what_the_kernel_counts_as_one", async () => {
    // As in the kernel: an object that sees, or a trainer's battle script; a partner's script alone is no trainer.
    expect(isTrainerFigure({ script: 0, trainer_type: 1 })).toBe(true);
    expect(isTrainerFigure({ script: 3001, trainer_type: 0 })).toBe(true);
    expect(isTrainerFigure({ script: 4999, trainer_type: 0 })).toBe(true);
    expect(isTrainerFigure({ script: 5001, trainer_type: 0 })).toBe(false);
    expect(isTrainerFigure({ script: 12, trainer_type: 0 })).toBe(false);
    vi.mocked(getEvents).mockResolvedValue({
      ...events,
      objects: [object(0, { local_id: 1, script: 12, trainer_type: 1, sight: 3 }), object(1, { local_id: 2, script: 5001 })],
    });
    await act(async () => root.render(<FigureField header={3} localId={null} onChange={onChange} headers={[town]} revision={0} lockTrainers onError={vi.fn()} />));
    await flush();
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Figur wählen…"]')!.click());
    await flush();
    const [seer, partner] = options();
    expect(seer?.textContent).toContain(trainersTalk());
    expect(seer?.getAttribute("aria-disabled") ?? seer?.getAttribute("data-disabled")).not.toBeNull();
    expect(partner?.textContent).not.toContain("Trainer");
    expect(partner?.getAttribute("aria-disabled")).not.toBe("true");
  });
});
