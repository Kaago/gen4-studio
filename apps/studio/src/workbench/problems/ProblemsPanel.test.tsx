// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CheckProblemDto, ProblemDto } from "../../api";
import { fakeChecks, finding } from "../../checks/checksTesting";
import type { ChecksApi } from "../../checks/useChecks";
import type { Depth } from "../../ui/frame/shellState";
import { readCss, ruleBody } from "../../ui/cssSource";
import { ProblemsPanel, ProblemsTable } from "./ProblemsPanel";
import { ProblemsStore, findingId } from "./problems";
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
  let checks: ChecksApi;
  const goTo = vi.fn();
  const show = (depth: Depth = "bauen") => act(() => root.render(<ProblemsPanel context={{ problems: store, depth, goTo, checks }} />));
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    store = new ProblemsStore();
    checks = fakeChecks();
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
    act(() => checks.feed.update({ reported: true }));
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

describe("ProblemsPanel with findings of the checks", () => {
  let container: HTMLElement;
  let root: Root;
  let store: ProblemsStore;
  let checks: ChecksApi;
  const goTo = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    store = new ProblemsStore();
    checks = fakeChecks();
    goTo.mockReset();
    act(() => root.render(<ProblemsPanel context={{ problems: store, depth: "bauen", goTo, checks }} />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.body.innerHTML = "";
  });

  const target = { kind: "event", eventFile: 411, eventKind: "warp", index: 2 } as const;
  const dest = finding("warp.dest-warp", "error", { args: ["2", "414", "5", "411", "3"], target });
  const dest3 = finding("warp.dest-warp", "error", { key: "warp:3", label: "Eventdatei 0411 · Warp 3", args: ["3", "414", "6", "411", "3"], target: { ...target, index: 3 } });
  const never = finding("warp.never-fires", "info", { key: "warp:5", label: "Eventdatei 0411 · Warp 5", args: ["5", "8", "4", "0x00", "0x00"], target: { ...target, index: 5 } });
  const wide = finding("text.too-wide", "warning", {
    subject: { kind: "textArchive", index: 433 },
    key: "message:1:line:1",
    label: "Textarchiv 0433 · Nachricht 1 · Zeile 1",
    args: ["433", "1", "1", "240", "216"],
    target: { kind: "textArchive", index: 433, message: 1 },
    fixes: [{ id: "wrap", label: "Umbrechen", irreversible: false }],
  });
  const party = finding("trainer.no-party", "error", { subject: { kind: "trainer", index: 3 }, key: "party", label: "Trainer 0003 · Team", args: ["3", "1"], target: { kind: "trainer", index: 3 } });
  const message = finding("script.message-missing", "error", { subject: { kind: "scriptFile", index: 5 }, key: "line:4:message:9", label: "Scriptdatei 0005 · Zeile 4 · Nachricht 9", args: ["5", "1", "9", "433", "3"] });
  const unreadable = finding("asset.unreadable", "error", { subject: { kind: "project", index: 0 }, key: "unreadable:personal_data", label: "Projekt · nicht lesbar", args: ["personal data", "", "no archive"] });

  const regions = () => [...container.querySelectorAll<HTMLElement>("[role=region]")];
  const regionName = (r: HTMLElement) => document.getElementById(r.getAttribute("aria-labelledby") ?? "")?.textContent;
  const row = (f: CheckProblemDto) => container.querySelector<HTMLElement>(`[data-finding="${findingId(f)}"]`)!;
  const buttons = (scope: Element) => [...scope.querySelectorAll<HTMLButtonElement>("button")].map((b) => b.textContent);
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("groups_the_findings_by_area_and_code_each_with_its_count", () => {
    act(() => store.replaceChecks([dest, dest3, never, message, wide, party, unreadable]));
    expect(container.querySelector("header")?.textContent).toContain("5 Fehler, 1 Warnung, 1 Hinweis");
    // Welt, Scripts, Texte, Pokémon in this order (and the project's own files last), each head with its count.
    expect(regions().map((r) => regionName(r))).toEqual(["Welt3", "Scripts1", "Texte1", "Pokémon1", "Projekt1"]);
    const welt = regions()[0]!;
    const groups = [...welt.querySelectorAll<HTMLElement>("[role=group][data-check]")];
    expect(groups.map((g) => g.dataset.check)).toEqual(["warp.dest-warp", "warp.never-fires"]);
    expect(groups[0]!.querySelector("h4")?.textContent).toBe("Ziel-Warp fehlt2");
    expect([...groups[0]!.querySelectorAll<HTMLElement>("[data-finding]")].map((r) => [r.dataset.check, r.dataset.state])).toEqual([
      ["warp.dest-warp", "error"],
      ["warp.dest-warp", "error"],
    ]);
  });

  it("a_row_says_the_german_sentence_and_the_label_and_opens_its_target", async () => {
    act(() => store.replaceChecks([dest]));
    const r = row(dest);
    expect(r.textContent).toContain("Die Tür führt zu Warp 5 in Header 0414, aber dessen Eventdatei 0411 hat nur 3 Warps.");
    expect(r.textContent).toContain("Eventdatei 0411 · Warp 2");
    expect(r.querySelector("[role=img]")?.getAttribute("aria-label")).toBe("Fehler");
    await click(button("Öffnen", r));
    expect(goTo).toHaveBeenCalledWith(target);
  });

  it("offers_no_ausblenden_for_an_error_and_no_oeffnen_without_a_target", () => {
    act(() => store.replaceChecks([dest, message, never, wide]));
    expect(buttons(row(dest))).toEqual(["Öffnen"]);
    expect(buttons(row(message))).toEqual([]);
    expect(buttons(row(never))).toEqual(["Öffnen", "Ausblenden"]);
    expect(buttons(row(wide))).toEqual(["Öffnen", "Beheben…", "Ausblenden"]);
  });

  it("ausblenden_ignores_the_finding", async () => {
    act(() => store.replaceChecks([never]));
    await click(button("Ausblenden", row(never)));
    expect(checks.ignore).toHaveBeenCalledWith(expect.objectContaining({ code: "warp.never-fires", key: "warp:5", fingerprint: never.fingerprint }));
  });

  it("beheben_opens_the_fix_dialog_with_the_preview", async () => {
    vi.mocked(checks.planFix).mockResolvedValue({ changes: [{ asset: { kind: "TextArchive", index: 433 }, label: "Textarchiv 0433", before: "eine lange Zeile", after: "eine\nlange Zeile" }], irreversible: false });
    act(() => store.replaceChecks([wide]));
    await click(button("Beheben…", row(wide)));
    expect(checks.planFix).toHaveBeenCalledWith(expect.objectContaining({ code: "text.too-wide" }), "wrap");
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog.textContent).toContain("Textarchiv 0433 · Nachricht 1 · Zeile 1");
    expect(dialog.querySelector("[role=group][aria-label='Textarchiv 0433']")).not.toBeNull();
  });

  it("the_foot_says_how_many_are_ignored_and_reports_one_again", async () => {
    const ignored = Array.from({ length: 12 }, (_, i) => finding("warp.never-fires", "info", { key: `warp:${i}`, label: `Eventdatei 0411 · Warp ${i}` }));
    act(() => checks.feed.update({ reported: true, ignored }));
    const foot = container.querySelector<HTMLElement>("footer")!;
    expect(foot.textContent).toContain("12 ausgeblendet");
    const more = button("Wieder melden…", foot)!;
    expect(more.getAttribute("aria-expanded")).toBe("false");
    await click(more);
    expect(more.getAttribute("aria-expanded")).toBe("true");
    const list = container.querySelector<HTMLElement>("[role=list][aria-label=Ausgeblendet]")!;
    expect(list.querySelectorAll("[data-finding]")).toHaveLength(12);
    await click(button("Wieder melden", list.querySelector(`[data-finding="${findingId(ignored[3]!)}"]`)!));
    expect(checks.unignore).toHaveBeenCalledWith(ignored[3]);
  });

  it("says_not_checked_with_a_way_out_while_no_report_arrived_and_none_runs", async () => {
    // The first run failed or answered without a report: an empty list is no „Keine Probleme.“
    expect(container.textContent).toContain("Nicht geprüft.");
    expect(container.textContent).toContain("Erneut prüfen");
    expect(container.textContent).not.toContain("Keine Probleme.");
    await click(button("Erneut prüfen"));
    expect(checks.recheck).toHaveBeenCalledOnce();
    // Problems of a save are listed; a note says the checks have no result.
    act(() => store.replaceForFile("expanded/scripts/0042.rotom", [problem("expanded/scripts/0042.rotom", 7, "unbekannter Befehl")]));
    expect(container.querySelector("[data-unchecked]")?.textContent).toContain("Nicht geprüft");
    act(() => checks.feed.update({ reported: true }));
    expect(container.querySelector("[data-unchecked]")).toBeNull();
  });

  it("the_ignored_list_folds_when_it_empties_and_when_a_baseline_adds_to_it_and_scrolls_within_its_cap", async () => {
    const ignoredOf = (n: number) => Array.from({ length: n }, (_, i) => finding("warp.never-fires", "info", { key: `warp:${i}` }));
    const more = () => button("Wieder melden…", container.querySelector<HTMLElement>("footer")!);
    const list = () => container.querySelector<HTMLElement>("[role=list][aria-label=Ausgeblendet]");
    act(() => checks.feed.update({ reported: true, ignored: ignoredOf(1) }));
    expect(more()!.getAttribute("aria-expanded")).toBe("false");
    await click(more());
    expect(list()).not.toBeNull();
    // The last one is reported again: the list is empty and folds.
    act(() => checks.feed.update({ ignored: [] }));
    expect(more()).toBeUndefined();
    // „Ausgangslage übernehmen“ ignores 111 at once: the list stays folded.
    act(() => checks.feed.update({ ignored: ignoredOf(111) }));
    expect(more()!.getAttribute("aria-expanded")).toBe("false");
    expect(list()).toBeNull();
    // Open, entries taken off one by one keep it open; a baseline that adds to it folds it.
    await click(more());
    act(() => checks.feed.update({ ignored: ignoredOf(110) }));
    expect(more()!.getAttribute("aria-expanded")).toBe("true");
    expect(list()!.querySelectorAll("[data-finding]")).toHaveLength(110);
    act(() => checks.feed.update({ ignored: ignoredOf(140) }));
    expect(more()!.getAttribute("aria-expanded")).toBe("false");
    expect(list()).toBeNull();
    // Open, it scrolls within its own cap, and the foot takes at most half of the panel (the hand is often 250 px high).
    const css = readCss("./ProblemsPanel.module.css", import.meta.url);
    expect(ruleBody(css, ".ignoredList")).toMatch(/max-height:\s*calc\(var\(--g4-raum-64\)/);
    expect(ruleBody(css, ".ignoredList")).toMatch(/overflow:\s*auto/);
    expect(ruleBody(css, ".ignoredList")).toMatch(/min-height:\s*0/);
    expect(ruleBody(css, ".foot")).toMatch(/max-height:\s*50%/);
  });

  it("says_it_is_checking_instead_of_no_problems_before_the_first_report", () => {
    act(() => checks.feed.update({ running: true }));
    expect(container.textContent).toContain("Prüft das Projekt …");
    expect(container.textContent).not.toContain("Keine Probleme.");
    act(() => checks.feed.update({ running: false, reported: true }));
    expect(container.textContent).toContain("Keine Probleme.");
  });

  it("erneut_pruefen_checks_again_and_a_running_pass_shows_its_progress", async () => {
    await click(button("Erneut prüfen"));
    expect(checks.recheck).toHaveBeenCalledOnce();
    act(() => checks.feed.update({ running: true, progress: { label: "texts", done: 312, total: 724 } }));
    const status = container.querySelector<HTMLElement>("footer [role=status]")!;
    expect(status.textContent).toBe("Prüft … Texte 312/724");
    expect(button("Erneut prüfen")?.disabled).toBe(true);
  });

  it("says_what_was_not_checked", () => {
    act(() => checks.feed.update({ reported: true, skipped: [{ code: "script.message-missing", why: "scripts not adopted" }] }));
    const hints = container.querySelector<HTMLElement>("[role=list][aria-label='Nicht geprüft']")!;
    expect([...hints.querySelectorAll("li")].map((l) => [l.dataset.state, l.textContent])).toEqual([
      ["info", "Nachricht fehlt: nicht geprüft, die Scripts sind nicht übernommen. Übernimm sie unter Script, dann prüft das Studio sie."],
    ]);
    // Every run checks everything: nothing shown is older than the last report.
    expect(container.textContent).not.toContain("Stand der letzten vollen Prüfung");
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
