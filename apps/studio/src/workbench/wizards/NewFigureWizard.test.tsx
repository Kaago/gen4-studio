// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getObjectSprite, recipeApply, recipePlan, validateText } from "../../api";
import type { DraftDto, HeaderSummary, MatrixSummary, NewFigureReport, ProjectInfo, RecipeOutcome } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { pressKey } from "../../ui/primitives/testUtils";
import { NewFigureWizard } from "./NewFigureWizard";
import { DRAFT_DEBOUNCE_MS } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  recipePlan: vi.fn(),
  recipeApply: vi.fn(),
  validateText: vi.fn(async () => []),
  getObjectSprite: vi.fn(async () => null),
}));
// The place picker asks the app-wide cache for map pictures; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS + 250)));

const header = (id: number, internal: string, location: string, matrix: number, event: number): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: location, matrix_id: matrix, area_data_id: 0, event_file_id: event, text_archive_id: 554, script_file_id: 7, level_script_id: 0 });
const headers = [header(0, "T01", "Zweiblattdorf", 0, 5), header(4, "T01R0201", "Spielerhaus", 4, 9)];
const matrices: MatrixSummary[] = [{ index: 0, name: "m0", width: 2, height: 1, has_headers: true, has_altitudes: false, filled: 2 }];
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 5 };
// The plan as the kernel answers it: the figure on its tile in matrix 0 with the family's look, and its message.
const planned: DraftDto = {
  cells: [],
  tiles: [],
  events: [{ kind: "object", matrix: 0, x: 12, z: 7, look: 6, target: null }],
  messages: [{ archive: 554, text: "Hallo!" }],
  notes: ["Stellt Objekt 3 auf Kachel 12, 7 von Header 0.", "Script #12 in Scriptdatei 0007 sagt Nachricht 40 aus Textarchiv 0554."],
  irreversible: [],
};
const outcome: RecipeOutcome<NewFigureReport> = {
  report: { event_file: 5, object: 3, local_id: 4, script_file: 7, slot: 12, archive: 554, message: 40 },
  created: [],
  change: 17,
  changed: [
    { kind: "Event", index: 5 },
    { kind: "Script", index: 7 },
    { kind: "TextArchive", index: 554 },
  ],
  next: [
    { id: "edit_dialog", label: "Dialog bearbeiten", available: true, reason: null },
    { id: "story_join", label: "Zu einer Geschichte", available: false, reason: "Kommt mit 3d.7" },
  ],
};
const input = { header: 0, x: 12, z: 7, graphics_id: null, movement_type: 0, dir: 1, text: "Hallo!", hidden_flag: null };

describe("NewFigureWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  const onDone = vi.fn();
  const onShow = vi.fn();
  const onStep = vi.fn();
  const onCancel = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    vi.mocked(recipeApply).mockReset().mockResolvedValue(outcome);
    vi.mocked(validateText).mockClear();
    vi.mocked(getObjectSprite).mockClear();
    onDraft.mockReset();
    onDone.mockReset().mockResolvedValue(undefined);
    onShow.mockReset();
    onStep.mockReset();
    // The host takes the wizard away when it is left.
    onCancel.mockReset().mockImplementation(() => root.render(null));
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  async function show(prefill: { header?: number; tile?: { x: number; z: number } } = { header: 0, tile: { x: 12, z: 7 } }, depth: Depth = "erzaehlen") {
    await act(async () =>
      root.render(
        <NewFigureWizard depth={depth} headers={headers} matrices={matrices} project={project} prefill={prefill} onDraft={onDraft} onDone={onDone} onShow={onShow} onStep={onStep} onCancel={onCancel} onError={onError} />,
      ),
    );
    await flush();
  }
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  const textArea = () => container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Was sie sagt"]')!;
  async function write(text: string) {
    await act(async () => textArea().focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(textArea(), text);
      textArea().dispatchEvent(new Event("input", { bubbles: true }));
    });
  }

  it("builds_the_input_shows_the_draft_and_applies", async () => {
    await show();
    expect(container.querySelector('[data-wizard="new_figure"]')).not.toBeNull();
    expect(current()).toBe("1. Wo und wie");
    // The place and the tile come from the request (the selected place, the tile last chosen in the scene).
    expect(container.querySelector("[data-figure-place]")?.textContent).toContain("Zweiblattdorf");
    expect(field("Kachel x").value).toBe("12");
    expect(field("Kachel z").value).toBe("7");
    // The look is the family's figure until one is chosen; it stands still and faces down.
    expect(container.querySelector("[data-figure-look]")?.getAttribute("data-figure-look")).toBe("6");
    expect(button("Unten")?.getAttribute("aria-pressed")).toBe("true");
    // The draft from the first step on: before the text the plan sends a stand-in, so the scene shows the figure on its tile.
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_figure", { ...input, text: "..." });
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await click(button("Links"));
    await click(button("Unten"));
    await next();
    expect(current()).toBe("2. Dialog");
    expect(button("Weiter")?.hasAttribute("disabled") || button("Weiter")?.getAttribute("aria-disabled") === "true").toBe(true);
    await write("Hallo!");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("new_figure", input);
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    // The visibility flag: none until one is chosen.
    expect(container.querySelector('[data-flag-field="hidden_flag"]')?.textContent).toContain("Keins");
    await next();
    expect(current()).toBe("3. Zusammenfassung");
    const summary = container.textContent ?? "";
    expect(summary).toContain("Zweiblattdorf");
    expect(summary).toContain("Kachel x 12, z 7");
    expect(summary).toContain("„Hallo!“");
    // One undo step: no badge „Nicht rückgängig“.
    expect(container.textContent).not.toContain("Nicht rückgängig");
    await click(button("Aufstellen"));
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("new_figure", input);
    expect(onDone).toHaveBeenCalledExactlyOnceWith(outcome, input);
    // The result: what was made, in the kernel's numbers, and the next steps as the kernel names them.
    expect(container.textContent).toContain("Figur aufgestellt");
    expect(container.textContent).toContain("Objekt 3 in Eventdatei 0005");
    expect(container.textContent).toContain("Script #12, Nachricht 40 in Archiv 0554");
    const steps = [...container.querySelectorAll<HTMLButtonElement>("[data-next-steps] button[data-recipe]")];
    expect(steps.map((b) => [b.getAttribute("data-recipe"), b.disabled])).toEqual([
      ["edit_dialog", false],
      ["story_join", true],
    ]);
    // Made: the draft is gone.
    expect(onDraft).toHaveBeenLastCalledWith(null);
    await click(steps[0]);
    expect(onStep).toHaveBeenCalledExactlyOnceWith("edit_dialog", outcome, input);
    await click(button("Zeigen"));
    expect(onShow).toHaveBeenCalledExactlyOnceWith(outcome, input);
  });

  it("escape_clears_the_draft_and_a_refusal_stays_in_the_wizard", async () => {
    await show();
    await next();
    await write("Hallo!");
    await settle();
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await next();
    // The kernel refuses: the wizard says why and stays on its step; nothing was made.
    vi.mocked(recipeApply).mockRejectedValueOnce({ code: "error", message: "Auf 12, 7 steht schon Objekt 1", asset: null });
    await click(button("Aufstellen"));
    expect(container.textContent).toContain("Auf 12, 7 steht schon Objekt 1");
    expect(onDone).not.toHaveBeenCalled();
    expect(current()).toBe("3. Zusammenfassung");
    await act(async () => void container.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onCancel).toHaveBeenCalledWith(false);
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });

  it("before_apply_runs_first_and_a_failure_stops_the_apply", async () => {
    const order: string[] = [];
    const beforeApply = vi.fn(async () => void order.push("before"));
    vi.mocked(recipeApply).mockImplementation(async () => (order.push("apply"), outcome) as never);
    await act(async () =>
      root.render(
        <NewFigureWizard depth="erzaehlen" headers={headers} matrices={matrices} project={project} prefill={{ header: 0, tile: { x: 12, z: 7 } }} onDraft={onDraft} onDone={onDone} onCancel={onCancel} onError={onError} beforeApply={beforeApply} />,
      ),
    );
    await flush();
    await next();
    await write("Hallo!");
    await next();
    await click(button("Aufstellen"));
    expect(beforeApply).toHaveBeenCalledExactlyOnceWith(input);
    expect(order).toEqual(["before", "apply"]);

    // The open script document cannot be saved: nothing is applied, the app hears of the failure, the wizard stays.
    act(() => root.render(null));
    vi.mocked(recipeApply).mockClear();
    const failure = { code: "io", message: "Scriptdatei 0007 nicht schreibbar", asset: null };
    beforeApply.mockReset().mockRejectedValue(failure);
    await act(async () =>
      root.render(
        <NewFigureWizard depth="erzaehlen" headers={headers} matrices={matrices} project={project} prefill={{ header: 0, tile: { x: 12, z: 7 } }} onDraft={onDraft} onDone={onDone} onCancel={onCancel} onError={onError} beforeApply={beforeApply} />,
      ),
    );
    await flush();
    await next();
    await write("Hallo!");
    await next();
    await click(button("Aufstellen"));
    expect(recipeApply).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledExactlyOnceWith(failure);
    expect(current()).toBe("3. Zusammenfassung");
    expect(button("Aufstellen")?.disabled).toBe(false);
  });

  it("a_text_with_a_page_break_previews_two_pages", async () => {
    await show();
    await next();
    await write("Hallo!");
    // Shift+Enter starts a new page (▼, `\r`); the preview shows the DS box and can turn its pages.
    act(() => textArea().setSelectionRange(6, 6));
    pressKey(textArea(), "Enter", { shiftKey: true });
    expect(textArea().value).toBe("Hallo!\\r");
    await write("Hallo!\\rWie geht’s?");
    const preview = () => container.querySelector("[data-figure-preview]");
    expect(preview()?.textContent).toContain("Seite 1 von 2");
    expect(preview()?.textContent).toContain("Hallo!");
    await click(button("Nächste Seite"));
    expect(preview()?.textContent).toContain("Seite 2 von 2");
    expect(preview()?.textContent).toContain("Wie geht’s?");
    await settle();
    expect(vi.mocked(recipePlan).mock.lastCall?.[1]).toMatchObject({ text: "Hallo!\\rWie geht’s?" });
  });

  it("without_a_tile_from_the_scene_asks_for_one", async () => {
    await show({ header: 0 });
    expect(field("Kachel x").value).toBe("");
    expect(button("Weiter")?.hasAttribute("disabled") || button("Weiter")?.getAttribute("aria-disabled") === "true").toBe(true);
    await act(async () => {
      for (const [label, value] of [
        ["Kachel x", "3"],
        ["Kachel z", "4"],
      ]) {
        const input = field(label);
        input.focus();
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
        input.blur();
      }
    });
    await next();
    expect(current()).toBe("2. Dialog");
  });
});
