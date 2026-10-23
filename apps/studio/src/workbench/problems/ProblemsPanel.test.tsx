// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProblemDto } from "../../api";
import { ProblemsPanel } from "./ProblemsPanel";
import { ProblemsStore } from "./problems";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const problem = (file: string, line: number, message: string, severity: ProblemDto["severity"] = "error", source: ProblemDto["source"] = "compile"): ProblemDto => ({
  severity,
  file,
  line,
  col: 3,
  message,
  source,
});

describe("ProblemsPanel", () => {
  let container: HTMLElement;
  let root: Root;
  let store: ProblemsStore;
  const goTo = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    store = new ProblemsStore();
    goTo.mockReset();
    act(() => root.render(<ProblemsPanel context={{ problems: store, goTo }} />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const rows = () => [...container.querySelectorAll<HTMLElement>("tr[data-problem]")];

  it("says_there_are_no_problems", () => {
    expect(container.textContent).toContain("Keine Probleme");
  });

  it("lists_severity_file_line_message_and_source_and_follows_the_store", () => {
    act(() => {
      store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "unbekannter Befehl Bogus")]);
      store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "Zeile zu breit", "warning", "text")]);
    });
    expect(rows().map((r) => [...r.querySelectorAll("td")].map((td) => td.textContent))).toEqual([
      ["Fehler", "Scriptdatei 0042", "7", "unbekannter Befehl Bogus", "Compile"],
      ["Warnung", "Textarchiv 0199", "1", "Zeile zu breit", "Text"],
    ]);
    expect(container.querySelector("caption")?.textContent).toBe("1 Fehler, 1 Warnung");
    act(() => store.replaceForFile("expanded/scripts/0042.rotom", []));
    expect(rows()).toHaveLength(1);
  });

  it("a_click_on_a_script_problem_goes_to_the_file_and_line", () => {
    act(() => store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "x")]));
    act(() => rows()[0]!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "script", file: 42, line: 7 });
  });

  it("a_click_on_a_level_script_or_live_problem_goes_to_the_json_file", () => {
    act(() => store.replaceLive([{ severity: "warning", file: "expanded/scripts/0003.json", line: 2, col: 1, message: "live" }]));
    expect(rows()[0]!.querySelectorAll("td")[1]!.textContent).toBe("Level-Script 0003");
    expect(rows()[0]!.querySelectorAll("td")[4]!.textContent).toBe("live");
    act(() => rows()[0]!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "script", file: 3, line: 2 });
  });

  it("a_click_on_a_text_problem_goes_to_the_archive", () => {
    act(() => store.replaceText(199, [problem("expanded/textArchives/0199.json", 4, "Zeile zu breit", "warning", "text")]));
    act(() => rows()[0]!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 199 });
  });

  it("a_problem_of_an_unknown_file_is_shown_and_not_a_link", () => {
    act(() => store.replaceForFile("rotom.toml", [problem("rotom.toml", 1, "kaputt")]));
    expect(rows()[0]!.querySelectorAll("td")[1]!.textContent).toBe("rotom.toml");
    act(() => rows()[0]!.click());
    expect(goTo).not.toHaveBeenCalled();
  });
});
