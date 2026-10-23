import { describe, expect, it, vi } from "vitest";
import type { ProblemDto } from "../../api";
import { ProblemsStore, countsText, scriptFileOf, scriptPath, textArchiveOf } from "./problems";

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
    expect(store.counts()).toEqual({ errors: 0, warnings: 0 });
    store.replaceForFile(A, [problem(A, 1, "e"), problem(A, 2, "w", "warning")]);
    store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "t", "warning", "text")]);
    store.replaceLive([{ severity: "error", file: B, line: 4, col: 2, message: "live" }]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 2 });
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
    expect(store.counts()).toEqual({ errors: 2, warnings: 1 });
    // Once the compile row is gone (fixed), the live row shows again.
    store.replaceForFile(A, []);
    expect(store.list().map((p) => p.line)).toEqual([3, 4, 3]);
    expect(store.counts()).toEqual({ errors: 2, warnings: 1 });
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
    expect(store.counts()).toEqual({ errors: 0, warnings: 3 });
  });

  it("clear_forgets_every_source", () => {
    const store = new ProblemsStore();
    store.replaceForFile(A, [problem(A, 1, "c")]);
    store.replaceLive([{ severity: "error", file: A, line: 1, col: 1, message: "l" }]);
    store.clear();
    expect(store.list()).toEqual([]);
    expect(store.counts()).toEqual({ errors: 0, warnings: 0 });
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
    expect(countsText({ errors: 1, warnings: 0 })).toBe("1 Fehler");
    expect(countsText({ errors: 0, warnings: 1 })).toBe("1 Warnung");
    expect(countsText({ errors: 2, warnings: 1 })).toBe("2 Fehler, 1 Warnung");
    expect(countsText({ errors: 0, warnings: 0 })).toBe("");
  });
});
