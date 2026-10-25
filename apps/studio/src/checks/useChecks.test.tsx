// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { acceptBaseline, applyFix, checksStatus, ignoreFinding, runChecks, setCheckEnabled } from "../api";
import type { ChecksReportDto, ProjectInfo } from "../api";
import { createEditBus } from "../bus/editBus";
import type { EditBus } from "../bus/editBus";
import { ProblemsStore } from "../workbench/problems/problems";
import { finding, report } from "./checksTesting";
import { CHECKS_DEBOUNCE_MS, CHECKS_POLL_MS, replacesShown, useChecks } from "./useChecks";
import type { ChecksApi } from "./useChecks";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  runChecks: vi.fn(),
  checksStatus: vi.fn(),
  ignoreFinding: vi.fn(),
  unignoreFinding: vi.fn(),
  acceptBaseline: vi.fn(),
  setCheckEnabled: vi.fn(),
  planFix: vi.fn(),
  applyFix: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const project = (root: string) => ({ root }) as ProjectInfo;
const CHEAP = ["cheap"] as ChecksReportDto["groups"];

/** A promise and the function that settles it. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe("useChecks", () => {
  let container: HTMLElement;
  let root: Root;
  let bus: EditBus;
  let problems: ProblemsStore;
  let api: ChecksApi;
  const onError = vi.fn();
  const onFixed = vi.fn(async () => {});
  const onSave = vi.fn(async () => {});

  function Probe({ info }: { info: ProjectInfo | null }) {
    api = useChecks({ bus, problems, project: info, onError, onFixed, onSave });
    return null;
  }
  const render = (info: ProjectInfo | null) => act(async () => root.render(<Probe info={info} />));
  const wait = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));
  const codes = () => problems.list().map((p) => p.code);

  beforeEach(() => {
    vi.useFakeTimers();
    for (const fn of [runChecks, checksStatus, ignoreFinding, setCheckEnabled, applyFix, acceptBaseline]) vi.mocked(fn).mockReset();
    vi.mocked(checksStatus).mockResolvedValue(report({ groups: [], running: false }));
    onError.mockReset();
    onFixed.mockClear();
    onSave.mockClear();
    bus = createEditBus();
    problems = new ProblemsStore();
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
  });

  it("checks_everything_when_a_project_opens_and_writes_the_findings_as_source_check", async () => {
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp")], ignored: [finding("warp.never-fires", "info")] }));
    await render(null);
    expect(runChecks).not.toHaveBeenCalled();
    await render(project("/p"));
    expect(runChecks).toHaveBeenCalledWith("all");
    expect(problems.list().map((p) => [p.source, p.code])).toEqual([["check", "warp.dest-warp"]]);
    expect(api.feed.get().ignored.map((f) => f.code)).toEqual(["warp.never-fires"]);
    expect(api.feed.get().reported).toBe(true);
  });

  it("polls_the_status_while_a_pass_runs_and_shows_its_progress", async () => {
    const run = deferred<ChecksReportDto>();
    vi.mocked(runChecks).mockReturnValue(run.promise);
    vi.mocked(checksStatus).mockResolvedValue(report({ groups: [], running: true, progress: { label: "texts", done: 312, total: 724 } }));
    await render(project("/p"));
    // The spinner shows at once; the progress comes with the first look.
    expect(api.feed.get().running).toBe(true);
    expect(checksStatus).not.toHaveBeenCalled();
    await wait(CHECKS_POLL_MS);
    expect(checksStatus).toHaveBeenCalledTimes(1);
    expect(api.feed.get().progress).toEqual({ label: "texts", done: 312, total: 724 });
    await wait(CHECKS_POLL_MS);
    expect(checksStatus).toHaveBeenCalledTimes(2);
    vi.mocked(checksStatus).mockResolvedValue(report({ groups: [], running: false }));
    await act(async () => run.resolve(report({ findings: [finding("warp.dest-warp")] })));
    expect(api.feed.get().running).toBe(false);
    expect(api.feed.get().progress).toBeNull();
    expect(codes()).toEqual(["warp.dest-warp"]);
    const polls = vi.mocked(checksStatus).mock.calls.length;
    await wait(CHECKS_POLL_MS * 4);
    expect(checksStatus).toHaveBeenCalledTimes(polls);
  });

  it("checks_everything_300_ms_after_an_edit_once_for_a_burst_with_the_spinner", async () => {
    vi.mocked(runChecks).mockResolvedValue(report());
    await render(project("/p"));
    vi.mocked(runChecks).mockClear();
    act(() => {
      bus.emit({ topic: "edited" });
      bus.emit({ topic: "edited" });
    });
    await wait(CHECKS_DEBOUNCE_MS - 1);
    expect(runChecks).not.toHaveBeenCalled();
    act(() => bus.emit({ topic: "edited" }));
    await wait(CHECKS_DEBOUNCE_MS - 1);
    expect(runChecks).not.toHaveBeenCalled();
    // A full run: the text and script checks are cached per file in the kernel, a warm run reads only what changed.
    const run = deferred<ChecksReportDto>();
    vi.mocked(runChecks).mockReturnValueOnce(run.promise);
    await wait(1);
    expect(runChecks).toHaveBeenCalledTimes(1);
    expect(runChecks).toHaveBeenCalledWith("all");
    expect(api.feed.get().running).toBe(true);
    await act(async () => run.resolve(report({ generation: 2 })));
    expect(api.feed.get().running).toBe(false);
    // Other topics are not edits.
    act(() => bus.emit({ topic: "headerReveal" }));
    await wait(CHECKS_DEBOUNCE_MS);
    expect(runChecks).toHaveBeenCalledTimes(1);
  });

  it("every_report_replaces_the_findings_as_a_whole", async () => {
    const wide = finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1" });
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp"), wide], generation: 4 }));
    await render(project("/p"));
    expect(codes()).toEqual(["warp.dest-warp", "text.too-wide"]);
    // The text was wrapped and the warp fixed: the next run's report is all there is, nothing of the one before stays.
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("event.trigger-empty", "warning", { key: "trigger:0" })], generation: 5 }));
    act(() => bus.emit({ topic: "edited" }));
    await wait(CHECKS_DEBOUNCE_MS);
    expect(codes()).toEqual(["event.trigger-empty"]);
  });

  it("a_report_of_some_groups_only_is_no_result_and_asks_for_a_full_run", async () => {
    const wide = finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1" });
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp"), wide], generation: 4 }));
    await render(project("/p"));
    // A cheap report (someone else ran the cheap group, say) as the answer of switching a check on: it says nothing about the texts.
    const full = deferred<ChecksReportDto>();
    vi.mocked(runChecks).mockReturnValueOnce(full.promise);
    vi.mocked(setCheckEnabled).mockResolvedValue(report({ findings: [], groups: CHEAP, generation: 5 }));
    await act(async () => api.setEnabled("warp.never-fires", true));
    expect(codes()).toEqual(["warp.dest-warp", "text.too-wide"]);
    expect(runChecks).toHaveBeenLastCalledWith("all");
    await act(async () => full.resolve(report({ findings: [wide], generation: 5 })));
    expect(codes()).toEqual(["text.too-wide"]);
    expect(replacesShown(null, report({ groups: CHEAP }))).toBe(false);
  });

  it("takes_an_answer_without_groups_as_no_result_and_never_an_older_generation", async () => {
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp")], generation: 7 }));
    await render(project("/p"));
    // No report of the current state (the store changed during every pass): not "clean".
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [], groups: [], generation: 8 }));
    await act(async () => api.recheck());
    expect(codes()).toEqual(["warp.dest-warp"]);
    // An answer of an older state than the one shown.
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [], generation: 6 }));
    await act(async () => api.recheck());
    expect(codes()).toEqual(["warp.dest-warp"]);
    expect(replacesShown(null, report({ groups: [] }))).toBe(false);
    expect(replacesShown(7, report({ generation: 6 }))).toBe(false);
    expect(replacesShown(7, report({ generation: 7 }))).toBe(true);
    expect(replacesShown(null, report())).toBe(true);
  });

  it("drops_the_answer_of_a_project_that_was_replaced_meanwhile", async () => {
    const first = deferred<ChecksReportDto>();
    vi.mocked(runChecks).mockReturnValueOnce(first.promise).mockResolvedValue(report({ findings: [finding("trainer.no-party", "error", { subject: { kind: "trainer", index: 1 }, key: "party" })] }));
    await render(project("/a"));
    await render(project("/b"));
    await act(async () => first.resolve(report({ findings: [finding("warp.dest-warp")] })));
    expect(codes()).toEqual(["trainer.no-party"]);
    expect(runChecks).toHaveBeenCalledTimes(2);
  });

  it("ignoring_shows_the_answer_and_checks_everything_when_the_answer_lacks_the_findings_group", async () => {
    const wide = finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1" });
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [wide], generation: 2 }));
    await render(project("/p"));
    // The last report of the backend is a cheap one: its answer says nothing about the text warning and shows nothing; all is checked again.
    vi.mocked(ignoreFinding).mockResolvedValue(report({ groups: CHEAP, generation: 3 }));
    vi.mocked(runChecks).mockResolvedValue(report({ ignored: [wide], generation: 3 }));
    await act(async () => api.ignore(wide));
    expect(ignoreFinding).toHaveBeenCalledWith(wide);
    expect(runChecks).toHaveBeenLastCalledWith("all");
    expect(codes()).toEqual([]);
    expect(api.feed.get().ignored).toHaveLength(1);
  });

  it("switching_a_check_off_takes_its_findings_away_at_once", async () => {
    const empty = finding("encounter.empty-slot", "warning", { subject: { kind: "encounter", index: 3 }, key: "slot:grass:4:night" });
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [empty] }));
    await render(project("/p"));
    vi.mocked(setCheckEnabled).mockResolvedValue(report({ skipped: [{ code: "encounter.empty-slot", why: "disabled" }] }));
    await act(async () => api.setEnabled("encounter.empty-slot", false));
    expect(setCheckEnabled).toHaveBeenCalledWith("encounter.empty-slot", false);
    expect(codes()).toEqual([]);
    expect(api.feed.get().disabled).toEqual(["encounter.empty-slot"]);
  });

  it("a_fix_reloads_what_it_changed_and_checks_everything_again", async () => {
    const wide = finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1", fixes: [{ id: "wrap", label: "Umbrechen", irreversible: false }] });
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [wide] }));
    await render(project("/p"));
    vi.mocked(applyFix).mockResolvedValue({ changed: [{ kind: "TextArchive", index: 433 }], remaining: false });
    vi.mocked(runChecks).mockResolvedValue(report({ generation: 2 }));
    let result;
    await act(async () => void (result = await api.applyFix(wide, "wrap")));
    expect(result).toEqual({ changed: [{ kind: "TextArchive", index: 433 }], remaining: false });
    expect(applyFix).toHaveBeenCalledWith(wide, "wrap");
    expect(onFixed).toHaveBeenCalledWith([{ kind: "TextArchive", index: 433 }]);
    expect(runChecks).toHaveBeenLastCalledWith("all");
    expect(codes()).toEqual([]);
  });

  it("save_saves_the_project_as_the_title_bar_does", async () => {
    vi.mocked(runChecks).mockResolvedValue(report());
    await render(project("/p"));
    await act(async () => api.save());
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("reports_a_failed_run_and_forgets_everything_without_a_project", async () => {
    vi.mocked(runChecks).mockRejectedValueOnce({ code: "error", message: "no project open", asset: null });
    await render(project("/p"));
    expect(onError).toHaveBeenCalledOnce();
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp")] }));
    await act(async () => api.recheck());
    expect(codes()).toEqual(["warp.dest-warp"]);
    await render(null);
    expect(codes()).toEqual([]);
    expect(api.feed.get().reported).toBe(false);
  });

  it("a_fix_right_after_a_save_runs_the_checks_once_after_the_fix_also_when_it_outlasts_the_pause", async () => {
    vi.mocked(runChecks).mockResolvedValue(report());
    await render(project("/p"));
    vi.mocked(runChecks).mockClear();
    // „Speichern und anlegen“: the save announces an edit, then the fix runs (creating a script compiles, longer than the pause).
    const fix = deferred<{ changed: []; remaining: false }>();
    vi.mocked(applyFix).mockReturnValue(fix.promise);
    act(() => bus.emit({ topic: "edited" }));
    let applied: Promise<unknown> = Promise.resolve();
    act(() => void (applied = api.applyFix(finding("event.script-missing"), "create_script")));
    await wait(CHECKS_DEBOUNCE_MS * 3);
    expect(runChecks).not.toHaveBeenCalled();
    await act(async () => {
      fix.resolve({ changed: [], remaining: false });
      await applied;
    });
    await wait(CHECKS_DEBOUNCE_MS * 3);
    expect(runChecks).toHaveBeenCalledOnce();
  });

  it("a_fix_that_fails_still_runs_the_checks_an_edit_before_it_asked_for", async () => {
    vi.mocked(runChecks).mockResolvedValue(report());
    await render(project("/p"));
    vi.mocked(runChecks).mockClear();
    vi.mocked(applyFix).mockRejectedValue({ code: "error", message: "text archive 0554 has unsaved changes", asset: null });
    act(() => bus.emit({ topic: "edited" }));
    await act(async () => void (await api.applyFix(finding("event.script-missing"), "create_script").catch(() => {})));
    await wait(CHECKS_DEBOUNCE_MS * 3);
    expect(runChecks).toHaveBeenCalledOnce();
    // Without an edit waiting, a refused fix runs nothing.
    vi.mocked(runChecks).mockClear();
    await act(async () => void (await api.applyFix(finding("event.script-missing"), "create_script").catch(() => {})));
    await wait(CHECKS_DEBOUNCE_MS * 3);
    expect(runChecks).not.toHaveBeenCalled();
  });

  it("counts_the_baselines_taken_so_the_ignored_list_can_fold", async () => {
    vi.mocked(runChecks).mockResolvedValue(report());
    await render(project("/p"));
    expect(api.feed.get().baselines).toBe(0);
    const ignored = [finding("warp.never-fires", "info")];
    vi.mocked(ignoreFinding).mockResolvedValue(report({ ignored }));
    await act(async () => api.ignore(finding("warp.never-fires", "info")));
    expect(api.feed.get().baselines).toBe(0);
    vi.mocked(acceptBaseline).mockResolvedValue(report({ ignored: [...ignored, finding("warp.never-fires", "info", { key: "warp:7" })] }));
    await act(async () => api.acceptBaseline());
    expect(api.feed.get().baselines).toBe(1);
    expect(api.feed.get().ignored).toHaveLength(2);
    await render(null);
    expect(api.feed.get().baselines).toBe(0);
  });

  it("next_report_gives_the_findings_of_the_first_run_that_starts_after_it_was_asked_for", async () => {
    const old = finding("warp.dest-warp");
    const mapped = finding("map.model-invalid", "error", { subject: { kind: "map", index: 5 }, key: "model" });
    const running = deferred<ChecksReportDto>();
    vi.mocked(runChecks).mockReturnValueOnce(running.promise);
    await render(project("/p"));
    // A run under way when it is asked is of the state before (an import, say): its report does not count.
    let next: readonly unknown[] | null | undefined;
    act(() => void api.nextReport().then((findings) => (next = findings)));
    await act(async () => running.resolve(report({ findings: [old] })));
    expect(next).toBeUndefined();
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [old, mapped], generation: 2 }));
    act(() => bus.emit({ topic: "edited" }));
    await wait(CHECKS_DEBOUNCE_MS);
    expect(next).toEqual([old, mapped]);
    // A failed run, or another project, settles it with nothing.
    let failed: readonly unknown[] | null | undefined;
    act(() => void api.nextReport().then((findings) => (failed = findings)));
    vi.mocked(runChecks).mockRejectedValueOnce({ code: "error", message: "boom", asset: null });
    await act(async () => api.recheck());
    expect(failed).toBeNull();
    let gone: readonly unknown[] | null | undefined;
    act(() => void api.nextReport().then((findings) => (gone = findings)));
    await render(null);
    expect(gone).toBeNull();
  });
});
