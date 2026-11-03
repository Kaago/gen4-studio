// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { recipePlan, storyGet } from "../../api";
import type { ProjectInfo, StoryDto } from "../../api";
import { NOT_A_TEXT_PROJECT } from "../../flags/flagWords";
import { storyDto, storyRow } from "../storiesTesting";
import { StepRecipes } from "./StepRecipes";
import { StoryRecipeDialog } from "./StoryRecipeDialog";
import { NEEDS_VAR } from "./storyRecipeModel";
import type { StoryRecipeRequest } from "./storyRecipeModel";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), storyGet: vi.fn(), recipePlan: vi.fn(), getEvents: vi.fn() }));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 1 };

/** The detail's bar and the app's dialog: a click asks for a recipe, the dialog shows it. */
function Harness({ story, textProject }: { story: StoryDto; textProject: boolean }) {
  const [request, setRequest] = useState<StoryRecipeRequest | null>(null);
  return (
    <>
      <StepRecipes story={story} textProject={textProject} onRecipe={setRequest} />
      {request !== null && (
        <StoryRecipeDialog
          request={request}
          stories={[storyRow(1, { name: "Rivalin" }), storyRow(2, { name: "Anderes" })]}
          project={project}
          revision={0}
          headers={[]}
          onApplied={vi.fn()}
          onClose={() => setRequest(null)}
          onError={vi.fn()}
        />
      )}
    </>
  );
}

describe("StepRecipes", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(storyGet).mockReset().mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    vi.mocked(recipePlan).mockReset().mockResolvedValue({ cells: [], tiles: [], events: [], messages: [], notes: [], irreversible: [] });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const recipe = (id: string) => container.querySelector<HTMLButtonElement>(`button[data-story-recipe="${id}"]`);
  const reasonOf = (button: HTMLButtonElement | null) => document.getElementById(button?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("the_step_bar_opens_the_talk_dialog_with_the_story_prefilled", async () => {
    await act(async () => root.render(<Harness story={storyDto(1)} textProject />));
    // Four recipes, each a button of the bar.
    expect([...container.querySelectorAll("[data-story-recipe]")].map((b) => b.getAttribute("data-story-recipe"))).toEqual(["talk_advances", "figure_appears", "scene_on_enter", "reward_once"]);
    await act(async () => recipe("talk_advances")!.click());
    await flush();
    const dialog = document.querySelector<HTMLElement>('[data-story-recipe-dialog="talk_advances"]');
    expect(dialog).not.toBeNull();
    expect(dialog!.querySelector("h2")?.textContent).toBe("Gespräch schaltet weiter");
    // The story of the detail, and its highest step to start from.
    expect(dialog!.querySelector('[role=combobox][aria-label="Geschichte"]')?.textContent).toContain("Rivalin");
    expect(storyGet).toHaveBeenCalledWith("STORY_0001");
    expect(dialog!.querySelector('[role=combobox][aria-label="Ab Schritt"]')?.textContent).toContain("3: Pokédex bekommen");
    // One primary button: „Anlegen“, locked until the figure and the text are there.
    const apply = dialog!.querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!;
    expect(apply.textContent).toBe("Anlegen");
    expect(apply.disabled).toBe(true);
    expect(dialog!.querySelectorAll('button[data-kind="primary"]')).toHaveLength(1);
  });

  it("a_flag_story_and_a_dspre_folder_lock_the_recipes_with_the_reason", async () => {
    await act(async () => root.render(<Harness story={storyDto(1, { progress_number: 0x94 }, { progress: { flag: "FLAG_HAT_MEDAILLE" } })} textProject />));
    expect(recipe("talk_advances")!.disabled).toBe(true);
    expect(reasonOf(recipe("reward_once"))).toBe(NEEDS_VAR);
    await act(async () => root.render(<Harness story={storyDto(1)} textProject={false} />));
    expect(recipe("scene_on_enter")!.disabled).toBe(true);
    expect(reasonOf(recipe("scene_on_enter"))).toBe(NOT_A_TEXT_PROJECT);
  });
});
