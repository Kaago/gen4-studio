// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetId, OriginalStatus } from "../api";
import { MISSING, READY, resetSpy, withReset } from "../original/resetTesting";
import { changesWith, entry, report } from "./changesTesting";
import { ChangesSurface } from "./ChangesSurface";
import type { ChangesApi } from "./useChanges";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Two maps (one new), a trainer and a matrix that has no reset. */
const ENTRIES = [
  entry(),
  entry({ label: "Karte 0700", state: "new", target: { kind: "map", index: 700 }, assets: [{ kind: "Map", index: 700 }] }),
  entry({ group: "Trainer", label: "Trainer 0001", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], folder: "trainer_properties" }),
  entry({ group: "Matrizen", label: "Matrix 0000", target: null, assets: [], folder: "matrices" }),
];

describe("ChangesSurface", () => {
  let container: HTMLElement;
  let root: Root;
  let reset: ReturnType<typeof resetSpy>;
  const onOpenProject = vi.fn();

  beforeEach(() => {
    onOpenProject.mockClear();
    reset = resetSpy();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const render = ({ changes = changesWith(), status = READY, dirty = [] }: { changes?: ChangesApi | null; status?: OriginalStatus | null; dirty?: AssetId[] } = {}) =>
    act(async () => root.render(withReset(reset, <ChangesSurface changes={changes} status={status} dirty={dirty} onOpenProject={onOpenProject} />)));
  const surface = () => container.querySelector<HTMLElement>("[data-technik=aenderungen]");
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => b.textContent === name);
  const group = (name: string) => container.querySelector<HTMLElement>(`[data-changes-group="${name}"]`);
  const change = (label: string) => container.querySelector<HTMLElement>(`[data-change="${label}"]`);
  const badges = (row: HTMLElement | null) => [...(row?.querySelectorAll("[class*=badge]") ?? [])].map((b) => b.textContent);
  const resetButton = (label: string) => button("Auf Original zurücksetzen…", change(label)!);
  // The reason a locked button gives: the text its aria-describedby points to.
  const reason = (b: HTMLElement | undefined) => document.getElementById(b?.getAttribute("aria-describedby") ?? "")?.textContent;
  const isLocked = (b: HTMLElement | undefined) => b?.getAttribute("aria-disabled") === "true" && !(b as HTMLButtonElement).disabled;

  it("without_a_project_offers_to_open_one", async () => {
    await render({ changes: null, status: null });
    expect(surface()?.querySelector("h1")?.textContent).toBe("Änderungen");
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    expect(surface()?.textContent).toContain("Öffne ein Projekt, dann zeigt Technik hier, was vom Original abweicht.");
    await act(async () => button("Projekt öffnen")!.click());
    expect(onOpenProject).toHaveBeenCalledOnce();
  });

  it("without_a_base_names_the_reason_and_points_to_veroeffentlichen", async () => {
    await render({ changes: changesWith({ report: null }), status: MISSING });
    const notice = surface()?.querySelector("[role=status]");
    expect(notice?.textContent).toContain("Diesem Projekt ist keine Basis-ROM zugeordnet.");
    expect(surface()?.textContent).toContain("Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.");
    // Nothing to compare against: no button, no list.
    expect(button("Neu vergleichen")).toBeUndefined();
    expect(container.querySelector("[data-changes-group]")).toBeNull();
  });

  it("groups_entries_with_badges_and_counts", async () => {
    const compare = vi.fn(async () => {});
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }), compare }) });
    expect(surface()?.querySelector("h1")?.textContent).toBe("Änderungen");
    expect(surface()?.textContent).toContain("Gegenüber der Basis-ROM Platin DE (Testprojekt). Gezählt wird der gespeicherte Stand.");
    // One section per group, in the order of the report, each with its count.
    expect([...container.querySelectorAll("[data-changes-group]")].map((g) => g.getAttribute("data-changes-group"))).toEqual(["Karten", "Trainer", "Matrizen"]);
    expect(group("Karten")?.querySelector("h3")?.textContent).toBe("Karten2");
    expect(group("Trainer")?.querySelector("h3")?.textContent).toBe("Trainer1");
    expect([...group("Karten")!.querySelectorAll("[data-change]")].map((r) => r.getAttribute("data-change"))).toEqual(["Karte 0003", "Karte 0700"]);
    expect(change("Karte 0003")?.getAttribute("role")).toBe("option");
    expect(badges(change("Karte 0003"))).toEqual(["Geändert"]);
    expect(badges(change("Karte 0700"))).toEqual(["Neu"]);
    expect(change("Karte 0003")?.querySelector("[class*=warning]")).not.toBeNull();
    expect(change("Karte 0700")?.querySelector("[class*=info]")).not.toBeNull();
    // A changed entry with a target can be reset; a new one has no original.
    expect(isLocked(resetButton("Karte 0003"))).toBe(false);
    expect(resetButton("Karte 0003")?.getAttribute("data-kind")).toBe("secondary");
    expect(isLocked(resetButton("Karte 0700"))).toBe(true);
    expect(reason(resetButton("Karte 0700"))).toBe("Kein Original");
    // One button for the whole surface: „Neu vergleichen“, secondary.
    const again = button("Neu vergleichen")!;
    expect(again.getAttribute("data-kind")).toBe("secondary");
    expect(container.querySelectorAll("[data-kind=primary]")).toHaveLength(0);
    await act(async () => again.click());
    expect(compare).toHaveBeenCalledOnce();

    // A missing entry has no original either; its badge says it is missing.
    await render({ changes: changesWith({ report: report({ entries: [entry({ label: "Karte 0599", state: "missing", target: { kind: "map", index: 599 } })] }) }) });
    expect(badges(change("Karte 0599"))).toEqual(["Fehlt"]);
    expect(change("Karte 0599")?.querySelector("[class*=error]")).not.toBeNull();
    expect(reason(resetButton("Karte 0599"))).toBe("Kein Original");

    // Nothing differs.
    await render({ changes: changesWith({ report: report() }) });
    expect(surface()?.textContent).toContain("Alles wie in der Basis-ROM.");
    expect(container.querySelector("[data-changes-group]")).toBeNull();

    // While the compare runs: its progress; the first time a hint that it takes longer only then.
    await render({ changes: changesWith({ report: null, busy: true, progress: { done: 120, total: 4096 } }) });
    const progress = [...container.querySelectorAll("[role=status]")].find((s) => s.textContent?.startsWith("Vergleiche"));
    expect(progress?.textContent).toBe("Vergleiche 120 von 4096 Dateien …");
    expect(surface()?.textContent).toContain("Das erste Mal dauert länger; danach geht es schnell.");
    expect(button("Neu vergleichen")?.getAttribute("aria-disabled")).toBe("true");
    // A later compare keeps the list shown and has no such hint.
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }), busy: true, progress: { done: 1, total: 4 } }) });
    expect(surface()?.textContent).toContain("Vergleiche 1 von 4 Dateien …");
    expect(surface()?.textContent).not.toContain("Das erste Mal dauert länger");
    expect(change("Karte 0003")).not.toBeNull();

    // A failure: the German sentence.
    await render({ changes: changesWith({ report: null, error: "Die Basis-ROM fehlt." }) });
    expect(container.querySelector("[role=alert]")?.textContent).toContain("Die Basis-ROM fehlt.");
  });

  it("a_row_without_target_has_a_locked_button_with_its_reason", async () => {
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
    const locked = resetButton("Matrix 0000")!;
    expect(isLocked(locked)).toBe(true);
    expect(reason(locked)).toBe("Für diese Art gibt es kein Zurücksetzen.");
    expect(badges(change("Matrix 0000"))).toEqual(["Geändert"]);
  });

  it("a_locked_button_is_focusable_and_described_and_does_nothing", async () => {
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
    const locked = resetButton("Karte 0700")!;
    // Not `disabled` (that would take it out of the Tab order and hide the reason from the keyboard): aria-disabled, described by its reason.
    expect(locked.disabled).toBe(false);
    expect(locked.getAttribute("aria-disabled")).toBe("true");
    expect(reason(locked)).toBe("Kein Original");
    act(() => locked.focus());
    expect(document.activeElement).toBe(locked);
    await act(async () => locked.click());
    expect(reset.open).not.toHaveBeenCalled();
    expect(document.querySelector("[role=dialog]")).toBeNull();
    // A button that can be used has no such description.
    expect(resetButton("Karte 0003")?.hasAttribute("aria-describedby")).toBe(false);
    expect(resetButton("Karte 0003")?.hasAttribute("aria-disabled")).toBe(false);
  });

  describe("focus when the row that holds it disappears", () => {
    const trio = [entry({ label: "Karte 0001" }), entry({ label: "Karte 0002" }), entry({ label: "Karte 0003" })];
    const only = (...labels: string[]) => report({ entries: trio.filter((e) => labels.includes(e.label)) });
    const focusReset = (label: string) => act(() => resetButton(label)!.focus());

    it("goes_to_the_button_of_the_next_row", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0002");
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(change("Karte 0002")).toBeNull();
      expect(document.activeElement).toBe(resetButton("Karte 0003"));
    });

    it("goes_to_the_previous_row_when_the_last_one_goes", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0003");
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0002") }) });
      expect(document.activeElement).toBe(resetButton("Karte 0002"));
    });

    it("crosses_groups_and_ends_at_neu_vergleichen_when_nothing_is_left", async () => {
      await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      // The only row of its group: the first row of the next group.
      focusReset("Trainer 0001");
      await render({ changes: changesWith({ report: report({ entries: ENTRIES.filter((e) => e.label !== "Trainer 0001") }) }) });
      expect(document.activeElement).toBe(resetButton("Matrix 0000"));
      // The reset button of the row, not the fold button of the section head.
      expect(document.activeElement?.closest("h3")).toBeNull();
      // Nothing left: „Neu vergleichen“.
      focusReset("Matrix 0000");
      await render({ changes: changesWith({ report: report() }) });
      expect(document.activeElement).toBe(button("Neu vergleichen"));
    });

    it("a_reset_click_keeps_focus_and_the_refresh_after_it_moves_it", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      // The dialog closes and returns focus to the button; the refresh comes after.
      focusReset("Karte 0002");
      await act(async () => resetButton("Karte 0002")!.click());
      expect(document.activeElement).toBe(resetButton("Karte 0002"));
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(document.activeElement).toBe(resetButton("Karte 0003"));
    });

    it("survives_a_compare_that_shows_no_rows_and_then_goes_to_the_neighbour", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0002");
      // The compare runs and the surface shows only its progress: every row is gone, focus waits on „Neu vergleichen“ (never on the body).
      await render({ changes: changesWith({ report: null, busy: true, progress: { done: 3, total: 40 } }) });
      expect(change("Karte 0001")).toBeNull();
      expect(document.activeElement).toBe(button("Neu vergleichen"));
      expect(document.activeElement).not.toBe(document.body);
      // The new report lacks the row: the neighbour by the old order takes focus.
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(document.activeElement).toBe(resetButton("Karte 0003"));
      // The same when the surface showed „Wird gelesen …“ in between (no status, so no button: the title waits).
      focusReset("Karte 0003");
      await render({ status: null });
      expect(document.activeElement).toBe(surface()?.querySelector("h1"));
      await render({ changes: changesWith({ report: only("Karte 0001") }) });
      expect(document.activeElement).toBe(resetButton("Karte 0001"));
    });

    it("a_row_that_is_back_after_the_compare_keeps_focus_where_the_user_put_it", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0002");
      await render({ changes: changesWith({ report: null, busy: true }) });
      expect(document.activeElement).toBe(button("Neu vergleichen"));
      // A row still there after the compare: nothing is moved to a neighbour.
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      expect(change("Karte 0002")).not.toBeNull();
      expect(document.activeElement).toBe(button("Neu vergleichen"));
      // The user chose „Neu vergleichen“ himself before the row went: it stays.
      act(() => button("Neu vergleichen")!.focus());
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(document.activeElement).toBe(button("Neu vergleichen"));
    });

    it("a_row_that_comes_back_as_a_new_element_gets_focus_back_on_the_same_part", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0002");
      // The row's elements are replaced by new ones (what a remount does): the old button, which had focus, is gone.
      const old = change("Karte 0002")!;
      const fresh = old.cloneNode(true) as HTMLElement;
      act(() => old.replaceWith(fresh));
      expect(document.activeElement).toBe(document.body);
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      expect(change("Karte 0002")).toBe(fresh);
      expect(document.activeElement).toBe(button("Auf Original zurücksetzen…", fresh));
      // The row itself, when the row had focus.
      act(() => change("Karte 0001")!.focus());
      const row = change("Karte 0001")!;
      const again = row.cloneNode(true) as HTMLElement;
      act(() => row.replaceWith(again));
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      expect(document.activeElement).toBe(again);
    });

    it("the_dialog_asks_where_focus_goes_when_it_closes", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      await act(async () => resetButton("Karte 0002")!.click());
      const finalFocus = reset.open.mock.calls[0]![1]!.finalFocus!;
      // The row is still there: its button.
      expect(finalFocus()).toBe(resetButton("Karte 0002"));
      // The reset has taken it away before the dialog closes (focus is in the dialog meanwhile): the next row's button, not the section head.
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(finalFocus()).toBe(resetButton("Karte 0003"));
      // No rows at all: „Neu vergleichen“.
      await render({ changes: changesWith({ report: report() }) });
      expect(finalFocus()).toBe(button("Neu vergleichen"));
    });

    it("leaves_focus_alone_when_it_is_elsewhere_or_the_row_stays", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      focusReset("Karte 0002");
      await render({ changes: changesWith({ report: report({ entries: trio }), busy: true }) });
      expect(document.activeElement).toBe(resetButton("Karte 0002"));
      // The user moved on to „Neu vergleichen“ before the row went.
      act(() => button("Neu vergleichen")!.focus());
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(document.activeElement).toBe(button("Neu vergleichen"));
    });

    it("a_focused_row_goes_to_the_next_row", async () => {
      await render({ changes: changesWith({ report: report({ entries: trio }) }) });
      act(() => change("Karte 0002")!.focus());
      await render({ changes: changesWith({ report: only("Karte 0001", "Karte 0003") }) });
      expect(document.activeElement).toBe(change("Karte 0003"));
    });
  });

  it("an_unsaved_asset_is_marked", async () => {
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }), dirty: [{ kind: "Trainer", index: 1 }, { kind: "Map", index: 9 }] });
    expect(badges(change("Trainer 0001"))).toEqual(["Geändert", "Ungespeichert"]);
    expect(change("Trainer 0001")?.querySelector("[class*=unsaved]")).not.toBeNull();
    expect(badges(change("Karte 0003"))).toEqual(["Geändert"]);
  });

  it("reset_from_a_row_opens_the_dialog", async () => {
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
    await act(async () => resetButton("Trainer 0001")!.click());
    expect(reset.open).toHaveBeenCalledExactlyOnceWith({ kind: "trainer", index: 1 }, { finalFocus: expect.any(Function) });
    // A locked button opens nothing.
    await act(async () => resetButton("Matrix 0000")!.click());
    expect(reset.open).toHaveBeenCalledOnce();
  });

  it("unbuilt_folders_are_named", async () => {
    await render({ changes: changesWith({ report: report({ entries: ENTRIES, unbuilt: ["maps", "scripts"] }) }) });
    const notes = [...container.querySelectorAll("[role=status]")].map((n) => n.textContent);
    expect(notes).toContain("Noch nicht gebaut: maps, scripts. Nach dem nächsten Build geht der Vergleich schneller.");
    // Five are named; more are counted (a project never built lists every folder).
    const five = ["a", "b", "c", "d", "e"];
    await render({ changes: changesWith({ report: report({ entries: ENTRIES, unbuilt: five }) }) });
    expect([...container.querySelectorAll("[role=status]")].map((n) => n.textContent)).toContain(
      "Noch nicht gebaut: a, b, c, d, e. Nach dem nächsten Build geht der Vergleich schneller.",
    );
    await render({ changes: changesWith({ report: report({ entries: ENTRIES, unbuilt: [...five, "f", "g", "h"] }) }) });
    expect([...container.querySelectorAll("[role=status]")].map((n) => n.textContent)).toContain(
      "Noch nicht gebaut: a, b, c, d, e und 3 weitere. Nach dem nächsten Build geht der Vergleich schneller.",
    );
    await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
    expect(surface()?.textContent).not.toContain("Noch nicht gebaut");
  });
});
