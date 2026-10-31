// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { recipePlan } from "../../api";
import type { DraftDto, RecipeId } from "../../api";
import { DRAFT_DEBOUNCE_MS, useDraftPlan } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), recipePlan: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const draft = (note: string): DraftDto => ({ cells: [{ matrix: 3, x: 1, y: 0, source_map: 7 }], tiles: [], events: [], messages: [], notes: [note], irreversible: ["Karte 0007 wird kopiert"] });

/** A promise the test settles by hand: the plan's answer arrives when the test says so. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

describe("useDraftPlan", () => {
  let container: HTMLElement;
  let root: Root;
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  let seen: { draft: DraftDto | null; error: string | null } = { draft: null, error: null };

  function Probe({ recipe, input }: { recipe: RecipeId; input: unknown | null }) {
    seen = useDraftPlan(recipe, input, onDraft);
    return null;
  }
  const show = (input: unknown | null, recipe: RecipeId = "extend_area") => act(() => root.render(createElement(Probe, { recipe, input })));
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));

  beforeEach(() => {
    vi.useFakeTimers();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onDraft.mockReset();
    vi.mocked(recipePlan).mockReset();
    seen = { draft: null, error: null };
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("asks_once_after_the_debounce_and_reports_the_draft", async () => {
    const answer = draft("Erweitert Weltkarte 0003");
    vi.mocked(recipePlan).mockResolvedValue(answer);
    show({ matrix: 3, x: 1, y: 0, source_map: 7 });
    // Typing on: every change restarts the wait; nothing is asked before it ends.
    await wait(DRAFT_DEBOUNCE_MS - 1);
    show({ matrix: 3, x: 1, y: 0, source_map: 8 });
    await wait(DRAFT_DEBOUNCE_MS - 1);
    // A new object with the same values (a wizard builds its spec on every render) is the same input.
    show({ matrix: 3, x: 1, y: 0, source_map: 8 });
    expect(recipePlan).not.toHaveBeenCalled();
    await wait(1);
    expect(recipePlan).toHaveBeenCalledOnce();
    expect(recipePlan).toHaveBeenCalledWith("extend_area", { matrix: 3, x: 1, y: 0, source_map: 8 });
    expect(onDraft).toHaveBeenLastCalledWith(answer);
    expect(onDraft).toHaveBeenCalledOnce();
    expect(seen).toEqual({ draft: answer, error: null });
    // Incomplete input clears at once, without waiting.
    show(null);
    expect(onDraft).toHaveBeenLastCalledWith(null);
    expect(seen).toEqual({ draft: null, error: null });
    await wait(DRAFT_DEBOUNCE_MS * 2);
    expect(recipePlan).toHaveBeenCalledOnce();
  });

  it("a_late_plan_answer_after_cancel_is_dropped", async () => {
    const first = draft("erste Eingabe");
    vi.mocked(recipePlan).mockResolvedValueOnce(first);
    show({ source_map: 1 }, "create_room");
    await wait(DRAFT_DEBOUNCE_MS);
    expect(onDraft).toHaveBeenLastCalledWith(first);
    // A second input is asked for; the answer is still out when the wizard is cancelled (its input goes null).
    const late = deferred<DraftDto>();
    vi.mocked(recipePlan).mockReturnValueOnce(late.promise);
    show({ source_map: 2 }, "create_room");
    await wait(DRAFT_DEBOUNCE_MS);
    expect(recipePlan).toHaveBeenCalledTimes(2);
    show(null, "create_room");
    expect(onDraft).toHaveBeenLastCalledWith(null);
    await act(async () => late.resolve(draft("zweite Eingabe")));
    expect(onDraft).toHaveBeenLastCalledWith(null);
    expect(seen.draft).toBeNull();
    // An answer for an older input is dropped too: only the newest input's draft is shown.
    const older = deferred<DraftDto>();
    const newer = draft("dritte Eingabe");
    vi.mocked(recipePlan).mockReturnValueOnce(older.promise).mockResolvedValueOnce(newer);
    show({ source_map: 3 }, "create_room");
    await wait(DRAFT_DEBOUNCE_MS);
    show({ source_map: 4 }, "create_room");
    await wait(DRAFT_DEBOUNCE_MS);
    await act(async () => older.resolve(draft("veraltet")));
    expect(onDraft).toHaveBeenLastCalledWith(newer);
    expect(seen.draft).toBe(newer);
    // Closing the wizard (the hook goes away) clears the draft; an answer after that changes nothing.
    const after = deferred<DraftDto>();
    vi.mocked(recipePlan).mockReturnValueOnce(after.promise);
    show({ source_map: 5 }, "create_room");
    await wait(DRAFT_DEBOUNCE_MS);
    act(() => root.render(null));
    expect(onDraft).toHaveBeenLastCalledWith(null);
    const calls = onDraft.mock.calls.length;
    await act(async () => after.resolve(draft("nach dem Schließen")));
    expect(onDraft).toHaveBeenCalledTimes(calls);
  });

  it("a_failed_plan_reports_null_and_the_error", async () => {
    const answer = draft("Erweitert Weltkarte 0003");
    vi.mocked(recipePlan).mockResolvedValueOnce(answer).mockRejectedValueOnce({ code: "error", message: "Zelle x 1, y 0 ist nicht leer", asset: null });
    show({ source_map: 7 });
    await wait(DRAFT_DEBOUNCE_MS);
    expect(onDraft).toHaveBeenLastCalledWith(answer);
    show({ source_map: 8 });
    await wait(DRAFT_DEBOUNCE_MS);
    expect(onDraft).toHaveBeenLastCalledWith(null);
    expect(seen).toEqual({ draft: null, error: "Zelle x 1, y 0 ist nicht leer" });
    // The next answer takes the error away.
    vi.mocked(recipePlan).mockResolvedValueOnce(answer);
    show({ source_map: 9 });
    await wait(DRAFT_DEBOUNCE_MS);
    expect(seen).toEqual({ draft: answer, error: null });
  });
});
