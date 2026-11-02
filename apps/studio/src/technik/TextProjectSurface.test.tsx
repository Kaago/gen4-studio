// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { conflictDiff, resolveExternalConflict } from "../api";
import type { ConflictDiff } from "../api";
import { TextProjectSurface, requestTextProjectFocus } from "./TextProjectSurface";
import type { TextProjectView } from "./TextProjectSurface";
import { conflict, outcome, syncWith, textWith } from "./textProjectTesting";
import type { BrokenEntry } from "../api";
import { deleteSpy, withDelete } from "../delete/deleteTesting";
import { openSelect, typeInto } from "../ui/primitives/testUtils";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  conflictDiff: vi.fn(),
  resolveExternalConflict: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const DIFF: ConflictDiff = {
  lines: [
    { path: "power", studio: "40", dspre: "45" },
    { path: "pp", studio: "35", dspre: "30" },
  ],
  more: 3,
  unsaved: false,
};

describe("TextProjectSurface", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenProject = vi.fn();

  beforeEach(() => {
    onOpenProject.mockClear();
    vi.mocked(conflictDiff).mockReset().mockResolvedValue(DIFF);
    vi.mocked(resolveExternalConflict).mockReset().mockResolvedValue([{ kind: "Move", index: 1 }]);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const render = (text: TextProjectView | null) => act(async () => root.render(<TextProjectSurface text={text} onOpenProject={onOpenProject} />));
  const surface = () => container.querySelector<HTMLElement>("[data-technik=textprojekt]");
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => b.textContent === name);
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());
  const row = (key: string) => container.querySelector<HTMLElement>(`[data-conflict="${key}"]`);
  const badges = (scope: HTMLElement | null) => [...(scope?.querySelectorAll("[class*=badge]") ?? [])].map((b) => b.textContent);
  const reason = (b: HTMLElement | undefined) => document.getElementById(b?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("dspre_offers_set_up_and_locks_it_with_the_reason", async () => {
    const onSetUp = vi.fn();
    await render(textWith({ status: { state: "dspre", adoptable: true, reason: null }, onSetUp }));
    expect(surface()?.querySelector("h1")?.textContent).toBe("Textprojekt");
    expect(surface()?.textContent).toContain(
      "Das Projekt liegt im DSPRE-Layout. Als Textprojekt bekommt jeder Eintrag eine lesbare Datei unter data/, bereit für Git; DSPRE öffnet den Ordner weiter.",
    );
    const setUp = button("Als Textprojekt einrichten…");
    expect(setUp?.getAttribute("data-kind")).toBe("primary");
    expect(setUp?.hasAttribute("aria-disabled")).toBe(false);
    await click(setUp);
    expect(onSetUp).toHaveBeenCalledOnce();
    // Nothing of a sync in a DSPRE folder.
    expect(button("Jetzt abgleichen")).toBeUndefined();
    expect(container.textContent).not.toContain("Zuletzt übernommen");

    onSetUp.mockClear();
    await render(textWith({ status: { state: "dspre", adoptable: false, reason: "Kein Textprojekt für Diamant/Perl in Phase 4." }, onSetUp }));
    const locked = button("Als Textprojekt einrichten…");
    expect(locked?.getAttribute("aria-disabled")).toBe("true");
    expect(reason(locked)).toBe("Kein Textprojekt für Diamant/Perl in Phase 4.");
    expect(surface()?.textContent).toContain("Kein Textprojekt für Diamant/Perl in Phase 4.");
    await click(locked);
    expect(onSetUp).not.toHaveBeenCalled();
  });

  it("current_shows_schema_and_last_sync", async () => {
    const run = vi.fn(async () => outcome());
    const onRevert = vi.fn();
    const at = Date.now() - 4 * 60_000;
    await render(textWith({ status: { state: "current", schema: 1, last_sync_at: at }, sync: syncWith({ run }), onRevert }));
    expect(surface()?.textContent).toContain("Textprojekt, Schema-Version 1. Zuletzt abgeglichen vor 4 Minuten.");
    expect(button("Jetzt abgleichen")?.getAttribute("data-kind")).toBe("secondary");
    expect(button("Zurücknehmen…")?.getAttribute("data-kind")).toBe("secondary");
    await click(button("Jetzt abgleichen"));
    expect(run).toHaveBeenCalledOnce();
    await click(button("Zurücknehmen…"));
    expect(onRevert).toHaveBeenCalledOnce();
    // Nothing taken over yet.
    expect(surface()?.textContent).toContain("Zuletzt übernommen");
    expect(surface()?.textContent).toContain("Nichts von außen geändert.");
    expect(container.querySelector("[data-sync-conflicts]")).toBeNull();
    expect(container.querySelector("[data-sync-broken]")).toBeNull();

    // A full sync under way says so, with what it has done; „Jetzt abgleichen“ waits.
    run.mockClear();
    await render(textWith({ status: { state: "current", schema: 1, last_sync_at: null }, sync: syncWith({ run, running: true, progress: { done: 120, total: 6500 }, last: null }) }));
    expect(surface()?.textContent).toContain("Noch nicht abgeglichen.");
    const status = [...container.querySelectorAll("[role=status]")].map((e) => e.textContent);
    expect(status).toContain("Gleiche mit DSPRE ab … 120 von 6500\u00a0Dateien");
    expect(surface()?.textContent).toContain("Das erste Mal dauert länger; danach geht es schnell.");
    expect(button("Jetzt abgleichen")?.getAttribute("aria-disabled")).toBe("true");
    await click(button("Jetzt abgleichen"));
    expect(run).not.toHaveBeenCalled();

    // What the last takeover brought, by side; a failed sync names why.
    const taken = outcome({
      adopted: [{ key: "moves/0001", label: "Attacke 0001 Pfund", assets: [{ kind: "Move", index: 1 }] }],
      applied: [{ key: "species/0025", label: "Pokémon 0025 Pikachu", assets: [{ kind: "Personal", index: 25 }] }],
    });
    await render(textWith({ sync: syncWith({ taken, error: "data/ nicht lesbar" }) }));
    const recent = container.querySelector<HTMLElement>("[data-sync-taken]");
    expect(recent?.textContent).toContain("Attacke 0001 Pfund");
    expect(badges(recent)).toEqual(["aus DSPRE", "aus Datei"]);
    expect([...container.querySelectorAll("[role=alert]")].map((e) => e.textContent)).toEqual(["data/ nicht lesbar"]);
  });

  it("jetzt_abgleichen_is_locked_while_a_build_runs", async () => {
    const run = vi.fn(async () => outcome());
    await render(textWith({ busy: true, sync: syncWith({ run }) }));
    const button_ = button("Jetzt abgleichen");
    expect(button_?.getAttribute("aria-disabled")).toBe("true");
    await click(button_);
    expect(run).not.toHaveBeenCalled();
  });

  it("set_up_and_take_back_are_locked_while_a_build_or_a_set_up_runs", async () => {
    const onSetUp = vi.fn();
    const onRevert = vi.fn();
    await render(textWith({ status: { state: "dspre", adoptable: true, reason: null }, busy: true, onSetUp }));
    const setUp = button("Als Textprojekt einrichten…");
    expect(setUp?.getAttribute("aria-disabled")).toBe("true");
    await click(setUp);
    expect(onSetUp).not.toHaveBeenCalled();
    await render(textWith({ busy: true, onRevert }));
    const revert = button("Zurücknehmen…");
    expect(revert?.getAttribute("aria-disabled")).toBe("true");
    await click(revert);
    expect(onRevert).not.toHaveBeenCalled();
    // While nothing runs, both answer.
    await render(textWith({ onRevert }));
    expect(button("Zurücknehmen…")?.hasAttribute("aria-disabled")).toBe(false);
    await click(button("Zurücknehmen…"));
    expect(onRevert).toHaveBeenCalledOnce();
  });

  it("the_choices_of_a_conflict_are_locked_while_a_build_runs", async () => {
    await render(textWith({ busy: true, sync: syncWith({ last: outcome({ conflicts: [conflict(), conflict({ key: "species/0025", label: "Pokémon 0025", other: "files" })] }) }) }));
    await click(button("Attacke 0001 Pfund", row("moves/0001")!));
    await click(button("Pokémon 0025", row("species/0025")!));
    for (const [key, take] of [
      ["moves/0001", "DSPRE-Fassung übernehmen"],
      ["species/0025", "Geänderte Datei übernehmen"],
    ] as const) {
      for (const name of ["Studio-Fassung behalten", take]) {
        const choice = button(name, row(key)!);
        expect(choice?.getAttribute("aria-disabled")).toBe("true");
        await click(choice);
      }
    }
    expect(resolveExternalConflict).not.toHaveBeenCalled();
  });

  it("a_conflict_row_shows_the_diff_and_both_choices_resolve", async () => {
    const onResolved = vi.fn();
    const last = outcome({ conflicts: [conflict(), conflict({ key: "species/0025", label: "Pokémon 0025 Pikachu", assets: [{ kind: "Personal", index: 25 }], other: "files" })] });
    await render(textWith({ sync: syncWith({ last }), onResolved }));
    const section = container.querySelector<HTMLElement>("[data-sync-conflicts]");
    expect(section?.textContent).toContain("Auf zwei Seiten geändert");
    expect(row("moves/0001")?.textContent).toContain("Attacke 0001 Pfund");
    expect(badges(row("moves/0001"))).toEqual(["Konflikt"]);
    // Folded: no diff read yet.
    expect(conflictDiff).not.toHaveBeenCalled();
    const toggle = button("Attacke 0001 Pfund", row("moves/0001")!);
    expect(toggle?.getAttribute("aria-expanded")).toBe("false");
    await click(toggle);
    expect(toggle?.getAttribute("aria-expanded")).toBe("true");
    expect(conflictDiff).toHaveBeenCalledExactlyOnceWith("moves/0001");
    const table = row("moves/0001")!.querySelector("table");
    expect([...table!.querySelectorAll("th")].map((th) => th.textContent)).toEqual(["Feld", "Studio", "DSPRE"]);
    expect([...table!.querySelectorAll("tbody tr")].map((tr) => [...tr.querySelectorAll("td")].map((td) => td.textContent))).toEqual([
      ["power", "40", "45"],
      ["pp", "35", "30"],
    ]);
    expect(row("moves/0001")?.textContent).toContain("und 3\u00a0weitere Abweichungen");
    expect(row("moves/0001")?.textContent).not.toContain("verwirft");
    expect(button("Studio-Fassung behalten", row("moves/0001")!)?.getAttribute("data-kind")).toBe("secondary");
    expect(button("DSPRE-Fassung übernehmen", row("moves/0001")!)?.getAttribute("data-kind")).toBe("secondary");
    await click(button("Studio-Fassung behalten", row("moves/0001")!));
    expect(resolveExternalConflict).toHaveBeenCalledExactlyOnceWith("moves/0001", "studio");
    expect(onResolved).toHaveBeenCalledExactlyOnceWith([{ kind: "Move", index: 1 }]);

    // A source changed on disk: the other side is the file.
    vi.mocked(conflictDiff).mockResolvedValue({ lines: [{ path: "name", studio: '"Pikachu"', dspre: '"Pika"' }], more: 0, unsaved: false });
    await click(button("Pokémon 0025 Pikachu", row("species/0025")!));
    const other = row("species/0025")!;
    expect([...other.querySelectorAll("th")].map((th) => th.textContent)).toEqual(["Feld", "Studio", "Datei"]);
    expect(other.textContent).not.toContain("weitere Abweichung");
    expect(button("DSPRE-Fassung übernehmen", other)).toBeUndefined();
    vi.mocked(resolveExternalConflict).mockResolvedValue([{ kind: "Personal", index: 25 }]);
    await click(button("Geänderte Datei übernehmen", other));
    expect(resolveExternalConflict).toHaveBeenLastCalledWith("species/0025", "dspre");
    expect(onResolved).toHaveBeenLastCalledWith([{ kind: "Personal", index: 25 }]);

    // A refused choice says why in its row.
    vi.mocked(resolveExternalConflict).mockRejectedValue({ code: "error", message: "Für diese Einheit gibt es keinen Konflikt mehr.", asset: null });
    await click(button("Studio-Fassung behalten", row("moves/0001")!));
    expect(row("moves/0001")?.querySelector("[role=alert]")?.textContent).toBe("Für diese Einheit gibt es keinen Konflikt mehr.");
  });

  it("an_unsaved_conflicts_diff_names_its_studio_column_unsaved", async () => {
    // UI test 4.3b U3: the backend's Studio side of an unsaved unit is the unsaved value; the column says so.
    vi.mocked(conflictDiff).mockResolvedValue({ lines: [{ path: "personal.attack", studio: "57", dspre: "60" }], more: 0, unsaved: true });
    await render(textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict({ unsaved: true })] }) }) }));
    await click(button("Attacke 0001 Pfund", row("moves/0001")!));
    const table = row("moves/0001")!.querySelector("table");
    expect([...table!.querySelectorAll("th")].map((th) => th.textContent)).toEqual(["Feld", "Studio (ungespeichert)", "DSPRE"]);
    expect([...table!.querySelectorAll("tbody td")].map((td) => td.textContent)).toEqual(["personal.attack", "57", "60"]);
  });

  it("an_agent_setting_the_folder_up_is_shown_and_locks_set_up", async () => {
    // UI test 4.3b U4: while an agent sets the folder up the surface says so and offers nothing of its own.
    const onSetUp = vi.fn();
    await render(textWith({ status: { state: "dspre", adoptable: true, reason: null }, agentSetUp: true, onSetUp }));
    expect([...container.querySelectorAll("[role=status]")].map((e) => e.textContent)).toContain("Wird von einem Agenten eingerichtet …");
    const setUp = button("Als Textprojekt einrichten…");
    expect(setUp?.getAttribute("aria-disabled")).toBe("true");
    await click(setUp);
    expect(onSetUp).not.toHaveBeenCalled();
    // A text project whose set-up an agent still reports: „Zurücknehmen…“ waits too.
    const onRevert = vi.fn();
    await render(textWith({ agentSetUp: true, onRevert }));
    expect(button("Zurücknehmen…")?.getAttribute("aria-disabled")).toBe("true");
    await click(button("Zurücknehmen…"));
    expect(onRevert).not.toHaveBeenCalled();
  });

  it("an_unsaved_conflict_warns_before_dspre", async () => {
    await render(textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict({ unsaved: true })] }) }) }));
    expect(badges(row("moves/0001"))).toEqual(["Ungespeichert"]);
    await click(button("Attacke 0001 Pfund", row("moves/0001")!));
    const notice = row("moves/0001")!.querySelector("[role=status]");
    expect(notice?.textContent).toBe("„DSPRE-Fassung übernehmen“ verwirft die ungespeicherten Änderungen an diesem Eintrag.");
    // The warning stands above the choice.
    const take = button("DSPRE-Fassung übernehmen", row("moves/0001")!)!;
    expect(notice!.compareDocumentPosition(take) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("broken_files_name_file_line_column", async () => {
    const broken = [
      {
        key: "moves/0001",
        label: "Attacke 0001 Pfund",
        problems: [
          { severity: "error" as const, file: "data/moves/0001.json", line: 3, col: 12, message: "Unbekannter Schlüssel „pwr“." },
          { severity: "error" as const, file: "data/moves/0001.json", line: 7, col: 1, message: "Unbekannte ID: TYPE_FIRE2." },
          { severity: "error" as const, file: "data/moves/0001.json", line: 9, col: 5, message: "textproject.schema: kein gültiges JSON: `,` oder `}` wird erwartet." },
        ],
      },
    ];
    await render(textWith({ sync: syncWith({ last: outcome({ broken }) }) }));
    const section = container.querySelector<HTMLElement>("[data-sync-broken]");
    expect(section?.textContent).toContain("Fehlerhafte Dateien");
    expect(section?.textContent).toContain("Attacke 0001 Pfund");
    expect([...section!.querySelectorAll("code")].map((c) => c.textContent)).toEqual([
      "data/moves/0001.json:3:12: Unbekannter Schlüssel „pwr“.",
      "data/moves/0001.json:7:1: Unbekannte ID: TYPE_FIRE2.",
      // UI test 4.3b U6: without the kernel's check code.
      "data/moves/0001.json:9:5: kein gültiges JSON: `,` oder `}` wird erwartet.",
    ]);
  });

  it("without_a_project_offers_to_open_one", async () => {
    await render(null);
    expect(surface()?.querySelector("h1")?.textContent).toBe("Textprojekt");
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    expect(surface()?.textContent).toContain("Öffne ein Projekt, dann zeigt Technik hier sein Textprojekt.");
    await click(button("Projekt öffnen"));
    expect(onOpenProject).toHaveBeenCalledOnce();
  });

  // A4 of the acceptance 4.3b: focus never rests on the body in the flow of the external sync.
  describe("focus", () => {
    const BROKEN: BrokenEntry[] = [{ key: "moves/0002", label: "Attacke 0002 Karateschlag", problems: [{ severity: "error", file: "data/moves/0002.json", line: 1, col: 1, message: "kaputt" }] }];
    const TAKEN = [{ key: "moves/0003", label: "Attacke 0003 Doppelschlag", assets: [{ kind: "Move" as const, index: 3 }] }];
    const toggleOf = (group: string) => container.querySelector<HTMLElement>(`[${group}] button[aria-expanded]`);
    const withAll = () => textWith({ sync: syncWith({ last: outcome({ conflicts: [conflict()], broken: BROKEN }), taken: outcome({ adopted: TAKEN }) }) });

    it("a_toast_action_before_the_surface_shows_focuses_the_group_asked_for", async () => {
      // The toasts navigate to Technik › Textprojekt: the surface mounts after the request.
      for (const [group, attribute] of [
        ["conflicts", "data-sync-conflicts"],
        ["broken", "data-sync-broken"],
        ["taken", "data-sync-taken"],
      ] as const) {
        await act(async () => root.render(null));
        (document.activeElement as HTMLElement | null)?.blur();
        requestTextProjectFocus(group);
        await render(withAll());
        expect(document.activeElement).toBe(toggleOf(attribute));
        expect(document.activeElement?.textContent).not.toBe("");
      }
    });

    it("a_toast_action_with_the_surface_already_shown_focuses_the_group", async () => {
      await render(withAll());
      expect(document.activeElement).toBe(document.body);
      await act(async () => requestTextProjectFocus("broken"));
      expect(document.activeElement).toBe(toggleOf("data-sync-broken"));
      await act(async () => requestTextProjectFocus("conflicts"));
      expect(document.activeElement).toBe(toggleOf("data-sync-conflicts"));
    });

    it("a_group_that_is_not_there_falls_back_to_the_surface_heading", async () => {
      requestTextProjectFocus("conflicts");
      await render(textWith());
      expect(document.activeElement).toBe(surface()?.querySelector("h1"));
      expect(document.activeElement?.textContent).toBe("Textprojekt");
      // The request is spent: a later render does not take focus again.
      (document.activeElement as HTMLElement).blur();
      await render(textWith());
      expect(document.activeElement).toBe(document.body);
    });

    const resolveRow = async (key: string, name: string, onResolved = vi.fn()) => {
      await click(button(row(key)!.querySelector("button")!.textContent!, row(key)!));
      const choice = button(name, row(key)!)!;
      act(() => choice.focus());
      await click(choice);
      return onResolved;
    };
    const keys = (n: number) => ["moves/0001", "species/0025", "moves/0004"].slice(0, n);
    const many = (n: number) =>
      outcome({ conflicts: keys(n).map((key, i) => conflict({ key, label: `Eintrag ${i + 1}`, assets: [{ kind: "Move", index: i + 1 }] })) });
    const rowToggle = (key: string) => row(key)?.querySelector<HTMLElement>("button[aria-expanded]");

    it("resolving_a_conflict_moves_focus_to_the_next_conflicts_toggle", async () => {
      await render(textWith({ sync: syncWith({ last: many(3) }) }));
      await resolveRow("moves/0001", "Studio-Fassung behalten");
      expect(resolveExternalConflict).toHaveBeenCalledOnce();
      // The sync lists what is still open: the resolved row is gone.
      await render(textWith({ sync: syncWith({ last: outcome({ conflicts: many(3).conflicts.slice(1) }) }) }));
      expect(row("moves/0001")).toBeNull();
      expect(document.activeElement).toBe(rowToggle("species/0025"));
    });

    it("resolving_the_last_conflict_moves_focus_to_the_previous_one", async () => {
      await render(textWith({ sync: syncWith({ last: many(3) }) }));
      await resolveRow("moves/0004", "DSPRE-Fassung übernehmen");
      await render(textWith({ sync: syncWith({ last: outcome({ conflicts: many(3).conflicts.slice(0, 2) }) }) }));
      expect(row("moves/0004")).toBeNull();
      expect(document.activeElement).toBe(rowToggle("species/0025"));
    });

    it("resolving_the_only_conflict_moves_focus_to_the_surface_heading", async () => {
      await render(textWith({ sync: syncWith({ last: many(1) }) }));
      await resolveRow("moves/0001", "Studio-Fassung behalten");
      await render(textWith({ sync: syncWith({ last: outcome() }) }));
      expect(container.querySelector("[data-sync-conflicts]")).toBeNull();
      expect(document.activeElement).toBe(surface()?.querySelector("h1"));
    });

    it("a_refused_choice_or_focus_the_user_moved_elsewhere_is_left_alone", async () => {
      await render(textWith({ sync: syncWith({ last: many(2) }) }));
      vi.mocked(resolveExternalConflict).mockRejectedValue({ code: "error", message: "Kein Konflikt mehr.", asset: null });
      await resolveRow("moves/0001", "Studio-Fassung behalten");
      expect(document.activeElement).toBe(button("Studio-Fassung behalten", row("moves/0001")!));
      // Resolved, then focus put elsewhere before the list changes.
      vi.mocked(resolveExternalConflict).mockResolvedValue([]);
      await click(button("Studio-Fassung behalten", row("moves/0001")!));
      const other = button("Jetzt abgleichen")!;
      act(() => other.focus());
      await render(textWith({ sync: syncWith({ last: outcome({ conflicts: many(2).conflicts.slice(1) }) }) }));
      expect(document.activeElement).toBe(other);
    });
  });

  it("the_delete_section_opens_the_dialog_for_a_matrix", async () => {
    const del = deleteSpy();
    const show = (text: TextProjectView, depth: "bauen" | "tuefteln") =>
      act(async () => root.render(withDelete(del, <TextProjectSurface text={text} onOpenProject={onOpenProject} depth={depth} />)));
    // Only in Tüfteln: the entities without a head of their own.
    await show(textWith(), "bauen");
    expect(container.querySelector("[data-delete-section]")).toBeNull();
    await show(textWith(), "tuefteln");
    const section = container.querySelector<HTMLElement>("[data-delete-section]");
    expect(section?.textContent).toContain("Löschen");
    const kind = section!.querySelector<HTMLElement>("[aria-label='Art']")!;
    expect(kind.textContent).toContain("Matrix");
    openSelect(kind);
    expect([...document.querySelectorAll<HTMLElement>("[role=option]")].map((o) => o.textContent)).toEqual(["Matrix", "Karte", "Eventdatei", "Gebietsdaten", "Scriptdatei", "Textarchiv"]);
    await act(async () => [...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent === "Matrix")!.click());
    const number = section!.querySelector<HTMLInputElement>("input[aria-label='Nummer']")!;
    await act(async () => number.focus());
    typeInto(number, "3");
    await act(async () => number.blur());
    await click(button("Löschen…", section!));
    expect(del.open).toHaveBeenLastCalledWith({ kind: "matrix", index: 3 });

    // A script file names its file.
    openSelect(kind);
    await act(async () => [...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent === "Scriptdatei")!.click());
    await click(button("Löschen…", section!));
    expect(del.open).toHaveBeenLastCalledWith({ kind: "script", file: 3 });

    // A map: its head's entry is locked by the very cell that shows it; here it can be deleted once no cell holds it.
    openSelect(kind);
    await act(async () => [...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent === "Karte")!.click());
    await click(button("Löschen…", section!));
    expect(del.open).toHaveBeenLastCalledWith({ kind: "map", index: 3 });

    // The DSPRE layout too: there a new entry at the end can have its number freed.
    await show(textWith({ status: { state: "dspre", adoptable: true, reason: null } }), "tuefteln");
    expect(container.querySelector("[data-delete-section]")).not.toBeNull();
  });
});
