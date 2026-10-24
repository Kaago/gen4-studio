// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProblemDto } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { ProblemsPanel, ProblemsTable } from "./ProblemsPanel";
import { ProblemsStore } from "./problems";
import type { ProblemRow } from "./problems";

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
  const show = (depth: Depth = "bauen") => act(() => root.render(<ProblemsPanel context={{ problems: store, depth, goTo }} />));
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    store = new ProblemsStore();
    goTo.mockReset();
    show();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const table = () => container.querySelector<HTMLElement>("[role=table]");
  const bodyRows = () => [...container.querySelectorAll<HTMLElement>("tbody [role=row]")];
  const cells = (row: HTMLElement) => [...row.querySelectorAll<HTMLElement>("[role=cell]")];
  const fileButton = (row: HTMLElement) => cells(row)[1]!.querySelector<HTMLButtonElement>("button")!;

  it("says_there_are_no_problems_in_a_head_and_an_empty_state", () => {
    expect(container.querySelector("h2")?.textContent).toBe("Probleme");
    expect(container.textContent).toContain("Keine Probleme.");
    expect(container.textContent).toContain("Beim Speichern eines Scripts oder Texts landen Fehler hier.");
    expect(table()).toBeNull();
  });

  it("lists_severity_file_position_message_and_source_in_a_table_and_follows_the_store", () => {
    act(() => {
      store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "Zeile zu breit", "warning", "text")]);
      store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "unbekannter Befehl Bogus")]);
    });
    expect(table()).not.toBeNull();
    expect([...container.querySelectorAll("th")].map((th) => th.textContent)).toEqual(["Schwere", "Datei", "Zeile:Spalte", "Meldung", "Quelle"]);
    const rows = bodyRows();
    // Errors first, whatever the file.
    expect(rows.map((r) => r.getAttribute("data-problem"))).toEqual(["error", "warning"]);
    expect(cells(rows[0]!).map((c) => c.textContent)).toEqual(["", "Scriptdatei 0042", "7:3", "unbekannter Befehl Bogus", "Compile"]);
    expect(cells(rows[1]!).map((c) => c.textContent)).toEqual(["", "Textarchiv 0199", "1:3", "Zeile zu breit", "Text"]);
    expect(cells(rows[0]!)[0]!.querySelector("[aria-label=Fehler]")).not.toBeNull();
    expect(cells(rows[1]!)[0]!.querySelector("[aria-label=Warnung]")).not.toBeNull();
    expect(container.querySelector("header")?.textContent).toContain("1 Fehler, 1 Warnung");
    act(() => store.replaceForFile("expanded/scripts/0042.rotom", []));
    expect(bodyRows()).toHaveLength(1);
  });

  it("a_click_on_the_file_reference_of_a_script_problem_goes_to_the_file_and_line", () => {
    act(() => store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "x")]));
    act(() => fileButton(bodyRows()[0]!).click());
    expect(goTo).toHaveBeenCalledWith({ kind: "script", file: 42, line: 7 });
  });

  it("a_level_script_or_live_problem_goes_to_the_json_file", () => {
    act(() => store.replaceLive([{ severity: "warning", file: "expanded/scripts/0003.json", line: 2, col: 1, message: "live" }]));
    const row = bodyRows()[0]!;
    expect(cells(row)[1]!.textContent).toBe("Level-Script 0003");
    expect(cells(row)[4]!.textContent).toBe("live");
    act(() => fileButton(row).click());
    expect(goTo).toHaveBeenCalledWith({ kind: "script", file: 3, line: 2 });
  });

  it("a_click_on_a_text_problem_goes_to_the_archive", () => {
    act(() => store.replaceText(199, [problem("expanded/textArchives/0199.json", 4, "Zeile zu breit", "warning", "text")]));
    act(() => fileButton(bodyRows()[0]!).click());
    expect(goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 199 });
  });

  it("a_trainer_problem_says_its_warning_in_german_and_opens_the_trainer", () => {
    const message = "trainer 12: the text before the battle does not end with \\r, so its box closes at once";
    act(() => store.replaceData("trainers/0012", [problem("trainers/0012", 1, message, "warning", "trainer")]));
    const row = bodyRows()[0]!;
    expect(cells(row).map((c) => c.textContent)).toEqual(["", "Trainer 0012", "1:3", "Der Text vor dem Kampf sollte auf ⏎ enden, sonst schließt sein Kasten sofort", "Trainer"]);
    act(() => fileButton(row).click());
    expect(goTo).toHaveBeenCalledWith({ kind: "trainer", index: 12 });
  });

  it("a_problem_of_an_unknown_file_is_shown_with_a_disabled_reference", () => {
    act(() => store.replaceForFile("rotom.toml", [problem("rotom.toml", 1, "kaputt")]));
    const button = fileButton(bodyRows()[0]!);
    expect(button.textContent).toBe("rotom.toml");
    expect(button.disabled).toBe(true);
    act(() => button.click());
    expect(goTo).not.toHaveBeenCalled();
  });

  it("tuefteln_shows_the_path_instead_of_the_label_in_the_file_column", () => {
    act(() => {
      store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "x")]);
      store.replaceText(199, [problem("expanded/textArchives/0199.json", 1, "y", "warning", "text")]);
    });
    show("tuefteln");
    expect(bodyRows().map((r) => cells(r)[1]!.textContent)).toEqual(["expanded/scripts/0042.rotom", "expanded/textArchives/0199.json"]);
    show("bauen");
    expect(bodyRows().map((r) => cells(r)[1]!.textContent)).toEqual(["Scriptdatei 0042", "Textarchiv 0199"]);
  });
});

describe("ProblemsTable", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("renders_the_given_rows_without_a_store_and_calls_goTo", () => {
    const goTo = vi.fn();
    const rows: ProblemRow[] = [{ severity: "error", file: "expanded/textArchives/0007.json", line: 2, col: 5, message: "kaputt", source: "text" }];
    act(() => root.render(<ProblemsTable rows={rows} depth="bauen" goTo={goTo} />));
    expect(container.querySelector("[role=table]")).not.toBeNull();
    act(() => container.querySelector<HTMLButtonElement>("tbody button")!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 7 });
  });
});
