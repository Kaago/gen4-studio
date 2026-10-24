import { describe, expect, it, vi } from "vitest";
import { encounterProblemFile, trainerProblemFile } from "../../api";
import type { ProblemDto } from "../../api";
import { finding } from "../../checks/checksTesting";
import { ProblemsStore, countsText, encounterFileOf, findingId, isFinding, problemTarget, scriptFileOf, scriptPath, textArchiveOf, trainerOf } from "./problems";

const problem = (file: string, line: number, message: string, severity: ProblemDto["severity"] = "error", source: ProblemDto["source"] = "compile"): ProblemDto => ({
  severity,
  file,
  line,
  col: 1,
  message,
  source,
});
const A = "expanded/scripts/0001.rotom";
const B = "expanded/scripts/0002.rotom";

describe("ProblemsStore", () => {
  it("replace_for_file_replaces_only_that_file", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 3, "a1"), problem(A, 5, "a2")]);
    store.replaceForFile(B, [problem(B, 1, "b1")]);
    store.replaceForFile(A, [problem(A, 9, "a3")]);
    expect(store.list().map((p) => `${p.file}:${p.line}`)).toEqual([`${A}:9`, `${B}:1`]);
    store.replaceForFile(A, []);
    expect(store.list().map((p) => p.message)).toEqual(["b1"]);
  });

  it("counts_errors_and_warnings_over_all_sources", () => {
    const store = new ProblemsStore();
    expect(store.counts()).toEqual({ errors: 0, warnings: 0, infos: 0 });
    store.replaceForFile(A, [problem(A, 1, "e"), problem(A, 2, "w", "warning")]);
    store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "t", "warning", "text")]);
    store.replaceLive([{ severity: "error", file: B, line: 4, col: 2, message: "live" }]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 2, infos: 0 });
    expect(countsText(store.counts())).toBe("2 Fehler, 2 Warnungen");
  });

  it("labels_each_row_with_its_source_and_lists_errors_first", () => {
    const store = new ProblemsStore();
    store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "t", "warning", "text")]);
    store.replaceLive([{ severity: "error", file: B, line: 4, col: 2, message: "live" }]);
    store.replaceForFile(A, [problem(A, 1, "c")]);
    expect(store.list().map((p) => [p.severity, p.source])).toEqual([
      ["error", "compile"],
      ["error", "live"],
      ["warning", "text"],
    ]);
  });

  it("replace_live_and_replace_text_replace_their_own_source_only", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 1, "c")]);
    store.replaceLive([{ severity: "warning", file: A, line: 2, col: 1, message: "l1" }]);
    store.replaceLive([{ severity: "warning", file: A, line: 3, col: 1, message: "l2" }]);
    expect(store.list().map((p) => p.message)).toEqual(["c", "l2"]);
    store.replaceLive([]);
    expect(store.list().map((p) => p.message)).toEqual(["c"]);
    const t = "expanded/textArchives/0007.json";
    store.replaceText(7, [problem(t, 1, "x", "warning", "text")]);
    store.replaceText(8, [problem("expanded/textArchives/0008.json", 1, "y", "warning", "text")]);
    store.replaceText(7, []);
    expect(store.list().map((p) => p.message)).toEqual(["c", "y"]);
  });

  it("notifies_subscribers_only_when_something_changed_and_keeps_the_snapshot_identity_otherwise", () => {
    const store = new ProblemsStore();
    const listener = vi.fn();
    const off = store.subscribe(listener);
    const empty = store.list();
    store.replaceForFile(A, []);
    store.replaceLive([]);
    expect(listener).not.toHaveBeenCalled();
    expect(store.list()).toBe(empty);
    store.replaceForFile(A, [problem(A, 1, "c")]);
    expect(listener).toHaveBeenCalledTimes(1);
    const first = store.list();
    const counts = store.counts();
    store.replaceForFile(A, [problem(A, 1, "c")]);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(store.list()).toBe(first);
    expect(store.counts()).toBe(counts);
    off();
    store.replaceForFile(A, []);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("lists_a_live_row_that_repeats_a_compile_row_once_and_counts_it_once", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 3, "unbekannter Befehl Bogus")]);
    // The server finds the same fault (other wording, other column) and one more on another line.
    store.replaceLive([
      { severity: "error", file: A, line: 3, col: 5, message: "unknown command Bogus" },
      { severity: "error", file: A, line: 4, col: 1, message: "other" },
      { severity: "warning", file: A, line: 3, col: 1, message: "other severity" },
    ]);
    expect(store.list().map((p) => [p.line, p.source, p.severity])).toEqual([
      [3, "compile", "error"],
      [4, "live", "error"],
      [3, "live", "warning"],
    ]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 1, infos: 0 });
    // Once the compile row is gone (fixed), the live row shows again.
    store.replaceForFile(A, []);
    expect(store.list().map((p) => p.line)).toEqual([3, 4, 3]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 1, infos: 0 });
  });

  it("apply_report_groups_by_file_and_archive_and_clears_the_named_ones", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 1, "old")]);
    store.replaceText(5, [problem("expanded/textArchives/0005.json", 1, "old text", "warning", "text")]);
    store.applyReport(
      [problem(B, 2, "b"), problem(B, 3, "b2", "warning"), problem("expanded/textArchives/0006.json", 1, "t6", "warning", "text"), problem("", 1, "no file", "warning", "text")],
      { files: [A], archives: [5] },
    );
    expect(store.list().map((p) => p.message)).toEqual(["b", "b2", "t6"]);
  });

  it("apply_build_report_drops_stale_compile_errors_but_keeps_warnings_and_replaces_named_files", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 1, "stale error"), problem(A, 2, "kept warning", "warning")]);
    store.replaceForFile(B, [problem(B, 1, "replaced", "warning")]);
    store.replaceText(5, [problem("expanded/textArchives/0005.json", 1, "text warning", "warning", "text")]);
    store.applyBuildReport([problem(B, 4, "fresh warning", "warning")]);
    expect(store.list().map((p) => p.message)).toEqual(["kept warning", "fresh warning", "text warning"]);
    expect(store.counts()).toEqual({ errors: 0, warnings: 3, infos: 0 });
  });

  it("replace_data_replaces_one_members_rows_and_never_piles_them_up", () => {
    const store = new ProblemsStore();
    const E = encounterProblemFile(12);
    const T = trainerProblemFile(928);
    store.replaceData(E, [problem(E, 1, "grass slot 0", "warning", "encounter"), problem(E, 3, "grass slot 2", "warning", "encounter")]);
    store.replaceData(T, [problem(T, 1, "trainer 928", "warning", "trainer")]);
    // Every read of the member replaces its rows (data_get, then after data_edit).
    store.replaceData(E, [problem(E, 3, "grass slot 2", "warning", "encounter")]);
    expect(store.list().map((p) => `${p.source}:${p.file}:${p.line}`)).toEqual(["encounter:encounters/0012:3", "trainer:trainers/0928:1"]);
    expect(store.counts()).toEqual({ errors: 0, warnings: 2, infos: 0 });
    store.replaceData(E, []);
    expect(store.list().map((p) => p.message)).toEqual(["trainer 928"]);
    expect(encounterFileOf(E)).toBe(12);
    expect(trainerOf(T)).toBe(928);
    expect(trainerOf(E)).toBeNull();
  });

  it("clear_forgets_every_source", () => {
    const store = new ProblemsStore();
    store.replaceData("trainers/0001", [problem("trainers/0001", 1, "t", "warning", "trainer")]);
    store.replaceForFile(A, [problem(A, 1, "c")]);
    store.replaceLive([{ severity: "error", file: A, line: 1, col: 1, message: "l" }]);
    store.replaceChecks([finding("warp.dest-warp")]);
    store.clear();
    expect(store.list()).toEqual([]);
    expect(store.counts()).toEqual({ errors: 0, warnings: 0, infos: 0 });
  });

  it("replace_checks_replaces_the_findings_as_a_whole_and_counts_infos", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 1, "c")]);
    const dest = finding("warp.dest-warp", "error");
    const wide = finding("text.too-wide", "warning", { subject: { kind: "textArchive", index: 433 }, key: "message:1:line:1" });
    const never = finding("warp.never-fires", "info", { key: "warp:5" });
    store.replaceChecks([dest, wide, never]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 1, infos: 1 });
    expect(countsText(store.counts())).toBe("2 Fehler, 1 Warnung, 1 Hinweis");
    // Errors, then warnings, then infos (each by file: a finding outside a source has none); every finding keeps its fields and says where it came from.
    expect(store.list().map((p) => [p.severity, p.source, p.code ?? null])).toEqual([
      ["error", "check", "warp.dest-warp"],
      ["error", "compile", null],
      ["warning", "check", "text.too-wide"],
      ["info", "check", "warp.never-fires"],
    ]);
    // The next run's findings replace all of them; the other sources stay.
    store.replaceChecks([never]);
    expect(store.list().map((p) => p.code ?? p.message)).toEqual(["c", "warp.never-fires"]);
    store.replaceChecks([]);
    expect(store.list().map((p) => p.message)).toEqual(["c"]);
  });

  it("the_same_findings_again_keep_the_snapshot_and_a_changed_one_replaces_it", () => {
    const store = new ProblemsStore();
    const listener = vi.fn();
    store.subscribe(listener);
    store.replaceChecks([finding("warp.dest-warp")]);
    expect(listener).toHaveBeenCalledTimes(1);
    const first = store.list();
    // Same code, subject and key (a copy, as the next run answers it): nothing changed.
    store.replaceChecks([finding("warp.dest-warp", "error", { message: "another wording of the kernel" })]);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(store.list()).toBe(first);
    // Another spot of the same subject, or the same spot with other content (its fingerprint), is another finding.
    store.replaceChecks([finding("warp.dest-warp", "error", { key: "warp:3" })]);
    expect(listener).toHaveBeenCalledTimes(2);
    store.replaceChecks([finding("warp.dest-warp", "error", { key: "warp:3", fingerprint: "00000000000000b2" })]);
    expect(listener).toHaveBeenCalledTimes(3);
  });

  it("names_a_finding_by_code_subject_and_key_and_tells_it_from_other_rows", () => {
    const f = finding("warp.dest-warp");
    expect(findingId(f)).toBe("warp.dest-warp|eventFile:411|warp:2");
    expect(isFinding(f)).toBe(true);
    expect(isFinding(problem(A, 1, "c"))).toBe(false);
  });
});

describe("problemTarget", () => {
  it("leads_a_finding_to_its_target_and_one_without_nowhere", () => {
    const target = { kind: "event", eventFile: 411, eventKind: "warp", index: 2 } as const;
    expect(problemTarget(finding("warp.dest-warp", "error", { target }))).toEqual(target);
    expect(problemTarget(finding("asset.unreadable", "error", { subject: { kind: "project", index: 0 }, key: "unreadable:personal_data" }))).toBeNull();
  });

  it("leads_a_script_text_trainer_or_encounter_row_by_its_path", () => {
    expect(problemTarget(problem("expanded/scripts/0042.rotom", 7, "x"))).toEqual({ kind: "script", file: 42, line: 7 });
    expect(problemTarget(problem("expanded/textArchives/0199.json", 1, "x", "warning", "text"))).toEqual({ kind: "textArchive", index: 199 });
    expect(problemTarget(problem("trainers/0012", 1, "x", "warning", "trainer"))).toEqual({ kind: "trainer", index: 12 });
    expect(problemTarget(problem("encounters/0012", 1, "x", "warning", "encounter"))).toEqual({ kind: "encounter", index: 12 });
    expect(problemTarget(problem("rotom.toml", 1, "x"))).toBeNull();
  });
});

describe("problem paths", () => {
  it("maps_script_and_text_paths_to_numbers_and_back", () => {
    expect(scriptPath(42, "script")).toBe("expanded/scripts/0042.rotom");
    expect(scriptPath(42, "levelScript")).toBe("expanded/scripts/0042.json");
    expect(scriptFileOf("expanded/scripts/0042.rotom")).toBe(42);
    expect(scriptFileOf("expanded/scripts/0042.json")).toBe(42);
    expect(scriptFileOf("expanded/textArchives/0042.json")).toBeNull();
    expect(textArchiveOf("expanded/textArchives/0199.json")).toBe(199);
    expect(textArchiveOf("expanded/scripts/0199.json")).toBeNull();
    expect(textArchiveOf("")).toBeNull();
  });

  it("writes_the_status_text_in_german", () => {
    expect(countsText({ errors: 1, warnings: 0, infos: 0 })).toBe("1 Fehler");
    expect(countsText({ errors: 0, warnings: 1, infos: 0 })).toBe("1 Warnung");
    expect(countsText({ errors: 2, warnings: 1, infos: 0 })).toBe("2 Fehler, 1 Warnung");
    expect(countsText({ errors: 2, warnings: 1, infos: 3 })).toBe("2 Fehler, 1 Warnung, 3 Hinweise");
    expect(countsText({ errors: 0, warnings: 0, infos: 1 })).toBe("1 Hinweis");
    expect(countsText({ errors: 0, warnings: 0, infos: 0 })).toBe("");
  });
});
