// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { recipeApply, releases, validateText } from "../api";
import type { HeaderSummary, NewFigureReport, RecipeOutcome } from "../api";
import { notATextProject } from "../flags/flagWords";
import { releasesOf } from "../publish/publishTesting";
import { textProjectSpy, withTextProject } from "../textproject/textProjectTesting";
import { ShellProvider } from "../ui/frame/ShellContext";
import type { WizardRequest } from "../workbench/WorkbenchContext";
import { WizardHost } from "./WizardHost";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  releases: vi.fn(),
  recipePlan: vi.fn(async () => ({ cells: [], tiles: [], events: [], messages: [], notes: [], irreversible: [] })),
  recipeApply: vi.fn(),
  validateText: vi.fn(async () => []),
  getObjectSprite: vi.fn(async () => null),
}));
// The place picker asks the app-wide cache for map pictures; there is no renderer here.
vi.mock("../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("WizardHost", () => {
  let container: HTMLElement;
  let root: Root;
  const publish = { gitOwn: true, onPublish: vi.fn(), onReveal: vi.fn(), onCommit: vi.fn(), onClose: vi.fn() };

  // A text project unless a test says otherwise (the story recipes need one).
  let textProject = true;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(releases).mockReset();
    for (const fn of Object.values(publish)) if (typeof fn === "function") fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    textProject = true;
  });

  function props(request: WizardRequest | null): ComponentProps<typeof WizardHost> {
    return {
      wizards: { request, open: vi.fn(), close: vi.fn(), created: vi.fn() },
      headers: [],
      matrices: [],
      world: null,
      matricesLoaded: true,
      matricesFailed: false,
      links: [],
      counts: {},
      selected: null,
      shownMatrix: null,
      lastTile: () => null,
      onHeaderPatch: async () => {},
      onError: vi.fn(),
      onPlay: vi.fn(),
      scripts: null,
      onAdoptScripts: vi.fn(),
      unsaved: 0,
      onSave: async () => {},
      onRegionApplied: async () => {},
      publish,
      onDraft: vi.fn(),
    };
  }
  const show = async (request: WizardRequest | null, over: Partial<ComponentProps<typeof WizardHost>> = {}) => {
    await act(async () =>
      root.render(
        withTextProject(
          textProjectSpy(textProject),
          <ShellProvider>
            <WizardHost {...props(request)} {...over} />
          </ShellProvider>,
        ),
      ),
    );
    await flush();
  };
  const field = (label: string) => container.querySelector<HTMLInputElement>(`[aria-label="${label}"]`);

  it("new_figure_saves_the_open_script_of_the_place_before_it_applies", async () => {
    const town: HeaderSummary = { id: 0, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 5, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };
    const outcome: RecipeOutcome<NewFigureReport> = {
      report: { event_file: 5, object: 3, local_id: 4, script_file: 7, slot: 12, archive: 554, message: 40 },
      created: [],
      change: 17,
      changed: [{ kind: "Script", index: 7 }],
      next: [],
    };
    const order: string[] = [];
    const scriptFiles = { saveIfDirty: vi.fn(async (file: number) => void order.push(`save ${file}`)), reload: vi.fn(async () => {}) };
    vi.mocked(recipeApply).mockReset().mockImplementation(async () => (order.push("apply"), outcome) as never);
    vi.mocked(validateText).mockClear();
    const onRecipeApplied = vi.fn(async () => {});
    const onError = vi.fn();
    const request: WizardRequest = { kind: "new_figure", prefill: { header: 0, tile: { x: 12, z: 7 } } };
    const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
    const click = (name: string) => act(async () => void button(name)?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const area = () => container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Was sie sagt"]')!;
    async function figure() {
      await show(request, { headers: [town], scriptFiles, onRecipeApplied, onError });
      await click("Weiter");
      await act(async () => area().focus());
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(area(), "Hallo!");
        area().dispatchEvent(new Event("input", { bubbles: true }));
      });
      await click("Weiter");
      await click("Aufstellen");
      await flush();
    }
    await figure();
    // Typing in the place's script file goes to disk first: the recipe appends the figure's slot to that file.
    expect(order).toEqual(["save 7", "apply"]);
    expect(onRecipeApplied).toHaveBeenCalledExactlyOnceWith(outcome.changed);

    // A save that fails stops the figure: nothing applied, the failure goes to the app.
    await show(null);
    order.length = 0;
    const failure = { code: "io", message: "Scriptdatei 0007 nicht schreibbar", asset: null };
    scriptFiles.saveIfDirty.mockRejectedValueOnce(failure);
    await figure();
    expect(order).toEqual([]);
    expect(recipeApply).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenCalledWith(failure);
  });

  it("story_join_after_a_new_figure_opens_the_talk_recipe_with_the_figure", async () => {
    const town: HeaderSummary = { id: 0, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 5, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };
    const outcome: RecipeOutcome<NewFigureReport> = {
      report: { event_file: 5, object: 3, local_id: 4, script_file: 7, slot: 12, archive: 554, message: 40 },
      created: [],
      change: 17,
      changed: [{ kind: "Event", index: 5 }],
      next: [
        { id: "edit_dialog", label: "Dialog bearbeiten", available: true, reason: null },
        { id: "story_join", label: "Zu einer Geschichte", available: true, reason: null },
      ],
    };
    vi.mocked(recipeApply).mockReset().mockResolvedValue(outcome as never);
    const order: string[] = [];
    const created = vi.fn(async () => void order.push("created"));
    const onOpenStoryRecipe = vi.fn(() => void order.push("recipe"));
    const base = props({ kind: "new_figure", prefill: { header: 0, tile: { x: 12, z: 7 } } });
    const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
    const click = (name: string) => act(async () => void button(name)?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await show(base.wizards.request, { headers: [town], wizards: { ...base.wizards, created }, onOpenStoryRecipe, onRecipeApplied: async () => {} });
    await click("Weiter");
    const area = container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Was sie sagt"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(area, "Hallo!");
      area.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await click("Weiter");
    await click("Aufstellen");
    await flush();
    const step = container.querySelector<HTMLButtonElement>('[data-next-steps] button[data-recipe="story_join"]')!;
    expect(step.disabled).toBe(false);
    await act(async () => step.click());
    await flush();
    // The figure's place is shown first (the wizard goes), then „Gespräch schaltet weiter“ opens with the new figure.
    expect(order).toEqual(["created", "recipe"]);
    expect(onOpenStoryRecipe).toHaveBeenCalledExactlyOnceWith({ recipe: "talk_advances", prefill: { figure: { header: 0, local_id: 4 } } });
    expect(base.wizards.open).not.toHaveBeenCalled();
  });

  it("story_join_outside_a_text_project_is_locked_with_the_reason", async () => {
    textProject = false;
    const town: HeaderSummary = { id: 0, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 5, text_archive_id: 554, script_file_id: 7, level_script_id: 0 };
    const outcome: RecipeOutcome<NewFigureReport> = {
      report: { event_file: 5, object: 3, local_id: 4, script_file: 7, slot: 12, archive: 554, message: 40 },
      created: [],
      change: 17,
      changed: [{ kind: "Event", index: 5 }],
      next: [
        { id: "edit_dialog", label: "Dialog bearbeiten", available: true, reason: null },
        { id: "story_join", label: "Zu einer Geschichte", available: true, reason: null },
      ],
    };
    vi.mocked(recipeApply).mockReset().mockResolvedValue(outcome as never);
    const onOpenStoryRecipe = vi.fn();
    const base = props({ kind: "new_figure", prefill: { header: 0, tile: { x: 12, z: 7 } } });
    const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
    const click = (name: string) => act(async () => void button(name)?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await show(base.wizards.request, { headers: [town], onOpenStoryRecipe, onRecipeApplied: async () => {} });
    await click("Weiter");
    const area = container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Was sie sagt"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(area, "Hallo!");
      area.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await click("Weiter");
    await click("Aufstellen");
    await flush();
    // A DSPRE folder: the story recipe could never go in, so the step is locked and says why.
    const step = container.querySelector<HTMLButtonElement>('[data-next-steps] button[data-recipe="story_join"]')!;
    expect(step.disabled).toBe(true);
    expect(container.querySelector('[data-next-steps] button[data-recipe="edit_dialog"]')?.hasAttribute("disabled")).toBe(false);
    expect(document.getElementById(step.getAttribute("aria-describedby") ?? "")?.textContent).toBe(notATextProject());
    await act(async () => step.click());
    expect(onOpenStoryRecipe).not.toHaveBeenCalled();
  });

  it("publish_request_renders_the_publish_wizard", async () => {
    vi.mocked(releases).mockResolvedValue(releasesOf({ next_version: "0.2.0" }));
    await show({ kind: "publish" });
    // The host reads the releases list when the wizard opens: the proposal is in the field.
    expect(releases).toHaveBeenCalledOnce();
    expect(container.querySelector('[role="dialog"]')?.getAttribute("aria-labelledby")).toBeTruthy();
    expect(container.textContent).toContain("Veröffentlichen");
    expect(field("Version")?.value).toBe("0.2.0");
    act(() => [...container.querySelectorAll("button")].find((b) => b.textContent === "Abbrechen")!.click());
    expect(publish.onClose).toHaveBeenCalledWith(false);

    // A list that cannot be read: its sentence, nothing to go on with.
    vi.mocked(releases).mockRejectedValue({ code: "releases_broken", message: "Die Liste der Veröffentlichungen ist nicht lesbar: kaputt", asset: null });
    await show(null);
    await show({ kind: "publish" });
    expect(container.querySelector("[role=alert]")?.textContent).toBe("Die Liste der Veröffentlichungen ist nicht lesbar: kaputt");
    expect([...container.querySelectorAll("button")].find((b) => b.textContent === "Weiter")?.disabled).toBe(true);
  });
});
