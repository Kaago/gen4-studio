// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { planRenameId, renameId } from "../api";
import type { RenamePlan, RenameResult } from "../api";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { PLAN_DELAY_MS, RenameIdDialog, renamedText } from "./RenameIdDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  planRenameId: vi.fn(),
  renameId: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const plan = (over: Partial<RenamePlan> = {}): RenamePlan => ({
  kind: "Trainer",
  number: 1,
  old_id: "TRAINER_JENS",
  new_id: "TRAINER_JENS_ROUTE201",
  files: [
    { path: "data/trainers/0001.json", count: 1 },
    { path: "g4-checks.json", count: 2 },
  ],
  total: 3,
  id_files: true,
  refusal: null,
  ...over,
});

describe("RenameIdDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onRenamed = vi.fn();

  const show = () => act(async () => root.render(<RenameIdDialog id="TRAINER_JENS" onClose={onClose} onRenamed={onRenamed} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const field = () => dialog()!.querySelector<HTMLInputElement>('input[aria-label="Neue ID"]')!;
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);
  const notices = (role: "status" | "alert") => [...(dialog()?.querySelectorAll<HTMLElement>(`[role=${role}]`) ?? [])].map((n) => n.textContent);
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));

  beforeEach(() => {
    vi.useFakeTimers();
    onClose.mockReset();
    onRenamed.mockReset();
    vi.mocked(planRenameId).mockReset().mockImplementation(async (id, newId) => plan({ old_id: id, new_id: newId }));
    vi.mocked(renameId).mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("plan_follows_the_input_after_a_pause", async () => {
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("ID umbenennen");
    // Filled with the old id; nothing is planned before an input.
    expect(field().value).toBe("TRAINER_JENS");
    await wait(PLAN_DELAY_MS * 2);
    expect(planRenameId).not.toHaveBeenCalled();
    expect(button("Umbenennen")?.disabled).toBe(true);
    // Typing: the plan waits for the pause after the last key.
    typeInto(field(), "TRAINER_JENS_R");
    await wait(PLAN_DELAY_MS - 50);
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS - 50);
    expect(planRenameId).not.toHaveBeenCalled();
    await wait(50);
    expect(planRenameId).toHaveBeenCalledOnce();
    expect(planRenameId).toHaveBeenCalledWith("TRAINER_JENS", "TRAINER_JENS_ROUTE201");
    expect(notices("status")).toEqual(["3\u00a0Verweise in 2\u00a0Dateien werden nachgezogen.data/trainers/0001.jsong4-checks.json"]);
    expect(button("Umbenennen")?.disabled).toBe(false);
    // Many files: eight are named, the rest counted.
    const files = Array.from({ length: 11 }, (_, i) => ({ path: `data/f${i}.json`, count: 1 }));
    vi.mocked(planRenameId).mockResolvedValueOnce(plan({ new_id: "TRAINER_JENS_X", files, total: 11 }));
    typeInto(field(), "TRAINER_JENS_X");
    await wait(PLAN_DELAY_MS);
    const listed = [...dialog()!.querySelectorAll("[data-rename-file]")].map((f) => f.textContent);
    expect(listed).toHaveLength(8);
    expect(dialog()?.textContent).toContain("und 3\u00a0weitere");
  });

  it("a_late_plan_for_older_input_is_not_shown", async () => {
    await show();
    let early: (p: RenamePlan) => void = () => {};
    vi.mocked(planRenameId)
      .mockImplementationOnce(() => new Promise((resolve) => (early = resolve)))
      .mockImplementationOnce(async (id, newId) => plan({ old_id: id, new_id: newId, total: 9 }));
    typeInto(field(), "TRAINER_A");
    await wait(PLAN_DELAY_MS);
    typeInto(field(), "TRAINER_B");
    await wait(PLAN_DELAY_MS);
    expect(planRenameId).toHaveBeenCalledTimes(2);
    expect(notices("status")[0]).toContain("9\u00a0Verweise");
    // The answer for „TRAINER_A“ comes last: it is dropped, the plan for „TRAINER_B“ stays.
    await act(async () => early(plan({ old_id: "TRAINER_JENS", new_id: "TRAINER_A", refusal: "Die ID TRAINER_A ist schon vergeben.", files: [], total: 0 })));
    expect(notices("alert")).toEqual([]);
    expect(notices("status")[0]).toContain("9\u00a0Verweise");
    expect(button("Umbenennen")?.disabled).toBe(false);
  });

  it("a_refusal_locks_rename", async () => {
    await show();
    vi.mocked(planRenameId).mockResolvedValueOnce(plan({ new_id: "TRAINER_RIVAL", refusal: "Die ID TRAINER_RIVAL ist schon vergeben.", files: [], total: 0 }));
    typeInto(field(), "TRAINER_RIVAL");
    await wait(PLAN_DELAY_MS);
    expect(notices("alert")).toEqual(["Die ID TRAINER_RIVAL ist schon vergeben."]);
    expect(button("Umbenennen")?.disabled).toBe(true);
    await act(async () => button("Umbenennen")!.click());
    expect(renameId).not.toHaveBeenCalled();
  });

  it("rename_says_undo_is_possible", async () => {
    const result: RenameResult = { plan: plan(), change: 7, ms: 120, warnings: [] };
    vi.mocked(renameId).mockResolvedValue(result);
    await show();
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    await act(async () => button("Umbenennen")!.click());
    expect(renameId).toHaveBeenCalledWith("TRAINER_JENS", "TRAINER_JENS_ROUTE201");
    expect(onRenamed).toHaveBeenCalledWith(result);
    expect(onClose).toHaveBeenCalled();
    expect(renamedText(result.plan)).toBe("ID umbenannt: TRAINER_JENS → TRAINER_JENS_ROUTE201. ⌘Z nimmt es zurück.");
  });

  it("a_pending_line_shows_while_the_plan_is_fetched", async () => {
    await show();
    let answer: (p: RenamePlan) => void = () => {};
    vi.mocked(planRenameId).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    expect(planRenameId).toHaveBeenCalledOnce();
    expect(notices("status")).toEqual(["Verweise werden gesucht …"]);
    expect(button("Umbenennen")?.disabled).toBe(true);
    await act(async () => answer(plan()));
    expect(notices("status")).toHaveLength(1);
    expect(notices("status")[0]).toContain("3\u00a0Verweise");
    expect(dialog()?.textContent).not.toContain("Verweise werden gesucht");
  });

  it("the_pending_line_ends_with_an_error_too", async () => {
    await show();
    vi.mocked(planRenameId).mockRejectedValueOnce(new Error("Datei nicht lesbar"));
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    expect(dialog()?.textContent).not.toContain("Verweise werden gesucht");
    expect(notices("alert")).toHaveLength(1);
  });

  it("a_late_plan_for_older_input_leaves_the_pending_line_of_the_newer_one", async () => {
    await show();
    let early: (p: RenamePlan) => void = () => {};
    vi.mocked(planRenameId)
      .mockImplementationOnce(() => new Promise((resolve) => (early = resolve)))
      .mockImplementationOnce(() => new Promise(() => {}));
    typeInto(field(), "TRAINER_A");
    await wait(PLAN_DELAY_MS);
    typeInto(field(), "TRAINER_B");
    await wait(PLAN_DELAY_MS);
    await act(async () => early(plan({ new_id: "TRAINER_A" })));
    expect(notices("status")).toEqual(["Verweise werden gesucht …"]);
  });

  it("rename_says_so_while_it_runs", async () => {
    let finish: (r: RenameResult) => void = () => {};
    vi.mocked(renameId).mockImplementation(() => new Promise((resolve) => (finish = resolve)));
    await show();
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    await act(async () => button("Umbenennen")!.click());
    expect(notices("status")).toContain("Wird umbenannt …");
    await act(async () => finish({ plan: plan(), change: 7, ms: 120, warnings: [] }));
    expect(onClose).toHaveBeenCalled();
  });

  it("enter_in_the_field_renames_when_ready", async () => {
    const result: RenameResult = { plan: plan(), change: 7, ms: 120, warnings: [] };
    vi.mocked(renameId).mockResolvedValue(result);
    await show();
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    const event = pressKey(field(), "Enter");
    await act(async () => {});
    expect(event.defaultPrevented).toBe(true);
    expect(renameId).toHaveBeenCalledOnce();
    expect(renameId).toHaveBeenCalledWith("TRAINER_JENS", "TRAINER_JENS_ROUTE201");
    expect(onRenamed).toHaveBeenCalledWith(result);
    expect(onClose).toHaveBeenCalled();
    // The field keeps the typed text (it does not fall back to the old id).
    expect(field().value).toBe("TRAINER_JENS_ROUTE201");
  });

  it("enter_does_nothing_while_the_plan_is_pending_or_refused_or_missing", async () => {
    await show();
    // Untouched field: no plan.
    pressKey(field(), "Enter");
    await act(async () => {});
    // Plan still being fetched.
    vi.mocked(planRenameId).mockImplementationOnce(() => new Promise(() => {}));
    typeInto(field(), "TRAINER_A");
    await wait(PLAN_DELAY_MS);
    pressKey(field(), "Enter");
    await act(async () => {});
    // Typed, the pause not over yet.
    typeInto(field(), "TRAINER_B");
    pressKey(field(), "Enter");
    await act(async () => {});
    // Refused.
    vi.mocked(planRenameId).mockResolvedValueOnce(plan({ new_id: "TRAINER_RIVAL", refusal: "Die ID TRAINER_RIVAL ist schon vergeben.", files: [], total: 0 }));
    typeInto(field(), "TRAINER_RIVAL");
    await wait(PLAN_DELAY_MS);
    pressKey(field(), "Enter");
    await act(async () => {});
    expect(renameId).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("enter_twice_renames_once", async () => {
    vi.mocked(renameId).mockImplementation(() => new Promise(() => {}));
    await show();
    typeInto(field(), "TRAINER_JENS_ROUTE201");
    await wait(PLAN_DELAY_MS);
    pressKey(field(), "Enter");
    await act(async () => {});
    pressKey(field(), "Enter");
    await act(async () => {});
    expect(renameId).toHaveBeenCalledOnce();
  });
});
