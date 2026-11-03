// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, recipeApply, recipePlan, storyGet, storyList } from "../api";
import type { HeaderSummary, ProjectInfo, RecipeOutcome, TalkAdvancesReport } from "../api";
import { storiesOf, storyDto, storyRow } from "../stories/storiesTesting";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { StoryRecipeHost } from "./StoryRecipeHost";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  storyList: vi.fn(),
  storyGet: vi.fn(),
  getEvents: vi.fn(),
  recipePlan: vi.fn(),
  recipeApply: vi.fn(),
}));
vi.mock("../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const wait = (ms: number) => act(() => new Promise<void>((resolve) => setTimeout(resolve, ms)));
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 1 };
const town: HeaderSummary = { id: 3, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 8, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

describe("StoryRecipeHost", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetToasts();
    vi.mocked(storyList).mockReset().mockResolvedValue(storiesOf([storyRow(1, { name: "Rivalin" })]));
    vi.mocked(storyGet).mockReset().mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    vi.mocked(getEvents).mockReset().mockResolvedValue({
      event_file_id: 8,
      locked: null,
      bg: [],
      warps: [],
      coords: [],
      objects: [{ index: 2, local_id: 4, graphics_id: 97, movement_type: 0, script: 0, dir: 0, trainer_type: 0, sight: 0, hidden_flag: 0, x: 1, z: 1, y: 0 }],
    });
    vi.mocked(recipePlan).mockReset().mockResolvedValue({
      cells: [],
      tiles: [],
      events: [],
      messages: [],
      notes: ["Figur 4 bekommt das Script #12; es springt danach zu ihrem bisherigen."],
      irreversible: ["Etwas bleibt nach Rückgängig."],
    });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
  });

  function props(over: Partial<ComponentProps<typeof StoryRecipeHost>> = {}): ComponentProps<typeof StoryRecipeHost> {
    return {
      request: { recipe: "talk_advances", prefill: { story: "STORY_0001", figure: { header: 3, local_id: 4 } } },
      onClose: vi.fn(),
      project,
      revision: 0,
      headers: [town],
      onApplied: vi.fn(async () => {}),
      onError: vi.fn(),
      ...over,
    };
  }

  it("applying_shows_the_plan_notes_and_then_the_toast", async () => {
    const outcome: RecipeOutcome<TalkAdvancesReport> = {
      report: { step: "s4", value: 4, event_file: 8, object: 2, script_file: 7, slot: 12, previous_slot: 0, archive: 554, message: 40 },
      created: [],
      change: 21,
      changed: [{ kind: "Event", index: 8 }, { kind: "Script", index: 7 }],
      next: [],
    };
    const applying = deferred<RecipeOutcome<TalkAdvancesReport>>();
    vi.mocked(recipeApply).mockReset().mockReturnValue(applying.promise as never);
    const host = props();
    await act(async () => root.render(<StoryRecipeHost {...host} />));
    await flush();
    const dialog = () => document.querySelector<HTMLElement>('[data-story-recipe-dialog="talk_advances"]')!;
    expect(dialog()).not.toBeNull();
    // The figure of the request, its text typed: the plan of the recipe shows its sentences on the right.
    expect(dialog().querySelector("[data-figure-pick]")?.textContent).toContain("Figur 4 · Aussehen 97");
    const text = dialog().querySelector<HTMLTextAreaElement>('textarea[aria-label="Text"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(text, "Hallo!");
      text.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await wait(350);
    await flush();
    expect(recipePlan).toHaveBeenLastCalledWith("talk_advances", { story: "STORY_0001", figure: { header: 3, local_id: 4 }, from_step: 3, text: "Hallo!", step_name: "Gespräch mit Figur 4" });
    const notes = dialog().querySelector<HTMLElement>("[data-story-recipe-notes]")!;
    expect(notes.textContent).toContain("Figur 4 bekommt das Script #12");
    expect(notes.querySelector("[data-story-recipe-irreversible] [role=status]")?.textContent).toContain("Etwas bleibt nach Rückgängig.");
    // „Anlegen“: busy while the recipe runs, then the toast names the step; the app reads again what changed and the dialog goes.
    const apply = dialog().querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!;
    expect(apply.disabled).toBe(false);
    await act(async () => apply.click());
    expect(dialog().textContent).toContain("Wird angelegt …");
    expect(dialog().querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!.disabled).toBe(true);
    expect(listToasts()).toHaveLength(0);
    await act(async () => applying.resolve(outcome));
    await flush();
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("talk_advances", expect.objectContaining({ text: "Hallo!", from_step: 3 }));
    expect(listToasts().map((t) => t.text)).toEqual(["Schritt „Gespräch mit Figur 4“ angelegt"]);
    expect(host.onApplied).toHaveBeenCalledExactlyOnceWith(outcome.changed);
    expect(host.onClose).toHaveBeenCalled();
  });

  it("a_refusal_stays_in_the_dialog_as_a_notice", async () => {
    vi.mocked(recipeApply).mockReset().mockRejectedValue({ code: "error", message: "Schritt 4 gibt es schon: „Kampf“.", asset: null });
    const host = props();
    await act(async () => root.render(<StoryRecipeHost {...host} />));
    await flush();
    const dialog = () => document.querySelector<HTMLElement>('[data-story-recipe-dialog="talk_advances"]')!;
    const text = dialog().querySelector<HTMLTextAreaElement>('textarea[aria-label="Text"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(text, "Hallo!");
      text.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await wait(350);
    await flush();
    await act(async () => dialog().querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!.click());
    await flush();
    expect(dialog().textContent).toContain("Schritt 4 gibt es schon: „Kampf“.");
    expect(host.onApplied).not.toHaveBeenCalled();
    expect(host.onClose).not.toHaveBeenCalled();
    expect(listToasts()).toHaveLength(0);
  });

  it("anlegen_waits_for_the_plan_of_the_changed_input", async () => {
    const host = props();
    await act(async () => root.render(<StoryRecipeHost {...host} />));
    await flush();
    const dialog = () => document.querySelector<HTMLElement>('[data-story-recipe-dialog="talk_advances"]')!;
    const apply = () => dialog().querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!;
    const type = (value: string) =>
      act(async () => {
        const text = dialog().querySelector<HTMLTextAreaElement>('textarea[aria-label="Text"]')!;
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(text, value);
        text.dispatchEvent(new Event("input", { bubbles: true }));
      });
    await type("Hallo!");
    // Everything is there, but no plan yet: locked.
    expect(apply().disabled).toBe(true);
    await wait(350);
    await flush();
    expect(apply().disabled).toBe(false);
    // The input changes: the last plan's sentences stay shown, but „Anlegen“ waits for the plan of the new input.
    const second = deferred<Awaited<ReturnType<typeof recipePlan>>>();
    vi.mocked(recipePlan).mockReturnValueOnce(second.promise);
    await type("Hallo, Reisender!");
    expect(dialog().querySelector("[data-story-recipe-notes]")?.textContent).toContain("Figur 4 bekommt das Script #12");
    expect(apply().disabled).toBe(true);
    await wait(350);
    expect(apply().disabled).toBe(true);
    await act(async () =>
      second.resolve({ cells: [], tiles: [], events: [], messages: [], notes: ["Neuer Plan."], irreversible: ["Level-Script-Datei 910 wird angelegt und bleibt"] }),
    );
    await flush();
    expect(dialog().querySelector("[data-story-recipe-irreversible]")?.textContent).toContain("Level-Script-Datei 910 wird angelegt und bleibt");
    expect(apply().disabled).toBe(false);
  });

  it("a_failed_story_list_shows_its_error_instead_of_reading_forever", async () => {
    vi.mocked(storyList).mockReset().mockRejectedValue({ code: "error", message: "data/ids/stories.json ist kaputt", asset: null });
    await act(async () => root.render(<StoryRecipeHost {...props({ request: { recipe: "scene_on_enter" } })} />));
    await flush();
    const dialog = document.querySelector<HTMLElement>('[data-story-recipe-dialog="scene_on_enter"]')!;
    expect(dialog.textContent).not.toContain("Die Geschichten werden gelesen …");
    expect(dialog.querySelector("[role=alert]")?.textContent).toContain("data/ids/stories.json ist kaputt");
  });
});
