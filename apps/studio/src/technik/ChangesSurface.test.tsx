// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { diffEntity } from "../api";
import type { Against, AssetId, ChangedEntry, DiffSubject, GitStatus, OriginalStatus } from "../api";
import { gitStatusOf, noRepo } from "../git/gitTesting";
import type { GitStatusApi } from "../git/useGitStatus";
import { MISSING, READY, readyWith, resetSpy, withReset } from "../original/resetTesting";
import { changesWith, entry, report } from "./changesTesting";
import { diffReport } from "./diffTesting";
import { ChangesSurface } from "./ChangesSurface";
import type { ChangesApi } from "./useChanges";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  diffEntity: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Two maps (one new), a trainer and a matrix that has no reset. */
const ENTRIES = [
  entry(),
  entry({ label: "Karte 0700", state: "new", target: { kind: "map", index: 700 }, assets: [{ kind: "Map", index: 700 }], subject: { kind: "map", index: 700 } }),
  entry({ group: "Trainer", label: "Trainer 0001", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], folder: "trainer_properties", subject: { kind: "trainer", index: 1 } }),
  entry({ group: "Matrizen", label: "Matrix 0000", target: null, assets: [], folder: "matrices", subject: { kind: "matrix", index: 0 } }),
];

describe("ChangesSurface", () => {
  let container: HTMLElement;
  let root: Root;
  let reset: ReturnType<typeof resetSpy>;
  const onOpenProject = vi.fn();

  beforeEach(() => {
    onOpenProject.mockClear();
    onAgainst.mockClear();
    onGitSetup.mockClear();
    onReloadUnsaved.mockClear();
    goTo.mockClear();
    vi.mocked(diffEntity).mockReset().mockResolvedValue(diffReport());
    reset = resetSpy();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const onAgainst = vi.fn();
  const onGitSetup = vi.fn();
  const goTo = vi.fn();
  const gitWith = (status: GitStatus | null, over: Partial<GitStatusApi> = {}): GitStatusApi => ({ status, error: null, refresh: vi.fn(async () => {}), ...over });
  const onReloadUnsaved = vi.fn();
  type RenderOptions = {
    changes?: ChangesApi | null;
    status?: OriginalStatus | null;
    dirty?: AssetId[];
    against?: Against;
    git?: GitStatusApi | null;
    unsaved?: ChangedEntry[] | null;
    unsavedError?: string | null;
  };
  const render = ({ changes = changesWith(), status = READY, dirty = [], against = "base", git = gitWith(gitStatusOf()), unsaved = null, unsavedError = null }: RenderOptions = {}) =>
    act(async () =>
      root.render(
        withReset(
          reset,
          <ChangesSurface
            changes={changes}
            status={status}
            dirty={dirty}
            onOpenProject={onOpenProject}
            against={against}
            onAgainst={onAgainst}
            git={git}
            unsaved={unsaved}
            unsavedError={unsavedError}
            onReloadUnsaved={onReloadUnsaved}
            onGitSetup={onGitSetup}
            goTo={goTo}
            revision={0}
          />,
        ),
      ),
    );
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

  describe("three comparisons", () => {
    const options = () => [...container.querySelectorAll<HTMLElement>("[data-against] button")];
    const notices = () => [...container.querySelectorAll("[role=status], [role=alert]")].map((n) => n.textContent);

    it("the_switch_has_three_options_and_starts_on_basis", async () => {
      await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      const toggle = container.querySelector<HTMLElement>("[data-against]")!;
      expect(toggle.getAttribute("data-against")).toBe("base");
      expect(toggle.querySelector("[aria-label]")?.getAttribute("aria-label")).toBe("Vergleichen mit");
      expect(options().map((o) => o.textContent)).toEqual(["Gegen Basis", "Gegen letzten Commit", "Gegen gespeichert"]);
      expect(options().map((o) => o.getAttribute("aria-pressed"))).toEqual(["true", "false", "false"]);
      await act(async () => options()[1]!.click());
      expect(onAgainst).toHaveBeenCalledExactlyOnceWith("head");
      // Without a base ROM the switch is there all the same: the last commit and the saved state need none.
      await render({ changes: changesWith({ report: null }), status: MISSING });
      expect(options()).toHaveLength(3);
      await act(async () => options()[2]!.click());
      expect(onAgainst).toHaveBeenLastCalledWith("saved");
    });

    it("head_without_a_repo_offers_git_setup", async () => {
      // The base ROM missing locks nothing here.
      await render({ against: "head", status: MISSING, git: gitWith(noRepo()) });
      expect(notices()).toContain("Dieses Projekt liegt in keinem Git-Repository.");
      expect(surface()?.textContent).toContain("Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.");
      expect(surface()?.textContent).not.toContain("Diesem Projekt ist keine Basis-ROM zugeordnet.");
      const setup = button("Git einrichten…")!;
      expect(setup.getAttribute("data-kind")).toBe("secondary");
      // Not a child of the surface (whose children take its reading width): the button keeps its own width.
      expect(setup.parentElement).not.toBe(surface());
      expect(setup.parentElement?.hasAttribute("data-git-offer")).toBe(true);
      await act(async () => setup.click());
      expect(onGitSetup).toHaveBeenCalledOnce();
      expect(container.querySelectorAll("[data-kind=primary]")).toHaveLength(0);
      // While the status is read: no offer yet.
      await render({ against: "head", git: gitWith(null) });
      expect(button("Git einrichten…")).toBeUndefined();
      expect(surface()?.textContent).toContain("Wird gelesen …");
    });

    it("head_without_a_commit_says_so", async () => {
      await render({ against: "head", git: gitWith(gitStatusOf({ head: null })) });
      expect(notices()).toContain("Noch kein Commit: es gibt nichts zu vergleichen.");
      expect(button("Git einrichten…")).toBeUndefined();
      expect(container.querySelector("[data-change]")).toBeNull();
    });

    it("head_lists_entries_with_geloescht_and_neu", async () => {
      const entries = [
        entry({ label: "Karte 0003" }),
        entry({ label: "Karte 0700", state: "new", target: { kind: "map", index: 700 }, assets: [{ kind: "Map", index: 700 }], subject: { kind: "map", index: 700 } }),
        entry({ group: "Trainer", label: "Trainer 0001", state: "missing", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], subject: { kind: "trainer", index: 1 } }),
      ];
      await render({ against: "head", status: MISSING, git: gitWith(gitStatusOf({ changes: 3, entries })), dirty: [{ kind: "Map", index: 3 }] });
      expect(surface()?.textContent).toContain("Gegenüber dem letzten Commit 4c1d2e3 „Ausgangsstand“ auf main. Gezählt wird der gespeicherte Stand.");
      expect(badges(change("Karte 0003"))).toEqual(["Geändert", "Ungespeichert"]);
      expect(badges(change("Karte 0700"))).toEqual(["Neu"]);
      expect(badges(change("Trainer 0001"))).toEqual(["Gelöscht"]);
      expect(reason(resetButton("Karte 0700"))).toBe("Kein Original");
      // „Neu vergleichen“ reads the status again.
      const git = gitWith(gitStatusOf({ entries }));
      await render({ against: "head", git });
      await act(async () => button("Neu vergleichen")!.click());
      expect(git.refresh).toHaveBeenCalledOnce();
      // Detached, in a larger repository.
      await render({ against: "head", git: gitWith(gitStatusOf({ entries, detached: true, branch: null, repo: { kind: "enclosing", workdir: "/Users/x" } })) });
      expect(surface()?.textContent).toContain("Repository /Users/x. Gegenüber dem letzten Commit 4c1d2e3 „Ausgangsstand“ (losgelöst). Gezählt wird der gespeicherte Stand.");
      // Nothing changed.
      await render({ against: "head", git: gitWith(gitStatusOf()) });
      expect(surface()?.textContent).toContain("Seit dem letzten Commit nichts geändert.");
      // Unreadable: the reason.
      await render({ against: "head", git: gitWith(null, { error: "Git-Repository nicht lesbar." }) });
      expect(container.querySelector("[role=alert]")?.textContent).toContain("Git-Repository nicht lesbar.");
    });

    it("saved_lists_the_unsaved_entries", async () => {
      const unsaved = [entry({ group: "Trainer", label: "Trainer 0001", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], subject: { kind: "trainer", index: 1 } })];
      await render({ against: "saved", status: MISSING, unsaved, dirty: [{ kind: "Trainer", index: 1 }] });
      expect(surface()?.textContent).toContain("Was ungespeichert vom gespeicherten Stand abweicht.");
      // One badge: the row is unsaved, nothing else.
      expect(badges(change("Trainer 0001"))).toEqual(["Ungespeichert"]);
      expect(button("Neu vergleichen")).toBeUndefined();
      await render({ against: "saved", unsaved: [] });
      expect(surface()?.textContent).toContain("Nichts ungespeichert.");
      await render({ against: "saved", unsaved: null });
      expect(surface()?.textContent).toContain("Wird gelesen …");
    });

    it("head_and_saved_rows_lock_reset_by_the_base_rom", async () => {
      const trainer = entry({ group: "Trainer", label: "Trainer 0001", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], subject: { kind: "trainer", index: 1 } });
      const map = entry({ label: "Karte 0700", target: { kind: "map", index: 700 }, assets: [{ kind: "Map", index: 700 }], subject: { kind: "map", index: 700 } });
      const fresh = entry({ label: "Karte 0701", state: "new", target: { kind: "map", index: 701 }, assets: [], subject: { kind: "map", index: 701 } });
      // Without a base ROM the lists show, but nothing can be reset.
      await render({ against: "head", status: MISSING, git: gitWith(gitStatusOf({ entries: [trainer, fresh] })) });
      expect(isLocked(resetButton("Trainer 0001"))).toBe(true);
      expect(reason(resetButton("Trainer 0001"))).toBe("Basis-ROM fehlt");
      expect(reason(resetButton("Karte 0701"))).toBe("Kein Original");
      await render({ against: "saved", status: MISSING, unsaved: [trainer] });
      expect(reason(resetButton("Trainer 0001"))).toBe("Basis-ROM fehlt");
      // While the base is read.
      await render({ against: "saved", status: null, unsaved: [trainer] });
      expect(reason(resetButton("Trainer 0001"))).toBe("Wird gelesen …");
      // Past the base's entries: no original.
      await render({ against: "head", status: readyWith({ map: 600, trainer: 900 }), git: gitWith(gitStatusOf({ entries: [trainer, map] })) });
      expect(isLocked(resetButton("Trainer 0001"))).toBe(false);
      expect(reason(resetButton("Karte 0700"))).toBe("Kein Original");
    });

    it("a_failed_unsaved_read_says_why_and_can_be_retried", async () => {
      await render({ against: "saved", unsaved: null, unsavedError: "Kein Projekt geöffnet." });
      expect(container.querySelector("[role=alert]")?.textContent).toContain("Kein Projekt geöffnet.");
      expect(surface()?.textContent).not.toContain("Wird gelesen …");
      await act(async () => button("Neu vergleichen")!.click());
      expect(onReloadUnsaved).toHaveBeenCalledOnce();
      // A list read before stays shown below the reason.
      await render({ against: "saved", unsaved: [entry()], unsavedError: "Kein Projekt geöffnet." });
      expect(container.querySelector("[role=alert]")).not.toBeNull();
      expect(change("Karte 0003")).not.toBeNull();
    });

    it("selecting_a_row_shows_its_diff", async () => {
      await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      const detail = () => container.querySelector<HTMLElement>("[data-entity-diff]");
      expect(detail()?.textContent).toBe("Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.");
      expect(diffEntity).not.toHaveBeenCalled();
      vi.mocked(diffEntity).mockResolvedValueOnce(diffReport({ against: "base", old_label: "Basis-ROM Platin DE (Testprojekt)" }));
      await act(async () => change("Trainer 0001")!.click());
      expect(change("Trainer 0001")?.getAttribute("aria-selected")).toBe("true");
      expect(diffEntity).toHaveBeenCalledExactlyOnceWith(ENTRIES[2]!.subject, "base");
      expect(detail()?.querySelector("h2")?.textContent).toBe("Trainer 0001");
      expect(detail()?.textContent).toContain("Basis-ROM Platin DE (Testprojekt) → Projekt (gespeichert)");
      // The list and the detail sit in a split that turns into two columns on a wide surface.
      expect(detail()?.closest("[data-changes-split]")).not.toBeNull();
      // Another comparison that lists the same entity keeps the choice (found by its subject, not its row).
      vi.mocked(diffEntity).mockClear();
      const moved = [{ ...ENTRIES[2]!, label: "Trainer 0001", state: "new" as const }, ENTRIES[0]!];
      await render({ against: "head", git: gitWith(gitStatusOf({ entries: moved })), changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      expect(change("Trainer 0001")?.getAttribute("aria-selected")).toBe("true");
      expect(diffEntity).toHaveBeenCalledExactlyOnceWith(ENTRIES[2]!.subject, "head");
      // One that does not list it forgets the choice, also for the comparison it came from.
      await render({ against: "saved", unsaved: [entry()], changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      expect(detail()?.textContent).toBe("Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.");
      await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      expect(change("Trainer 0001")?.getAttribute("aria-selected")).toBe("false");
      expect(detail()?.textContent).toBe("Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.");
    });

    it("a_list_still_read_keeps_the_choice_for_when_it_comes", async () => {
      await render({ changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      await act(async () => change("Trainer 0001")!.click());
      // „Gegen gespeichert“ while it is read: nothing to find the entity in, nothing forgotten.
      await render({ against: "saved", unsaved: null, changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      await render({ against: "saved", unsaved: [ENTRIES[2]!], changes: changesWith({ report: report({ entries: ENTRIES }) }) });
      expect(change("Trainer 0001")?.getAttribute("aria-selected")).toBe("true");
    });

    it("the_app_can_hold_the_choice_across_leaving_technik", async () => {
      // Held by the app (`chosen`, `onChoose`): a surface mounted anew (after „Öffnen“ and back) shows the entity chosen before.
      const onChoose = vi.fn();
      const show = (chosen: DiffSubject | null) =>
        act(async () =>
          root.render(
            withReset(
              reset,
              <ChangesSurface
                changes={changesWith({ report: report({ entries: ENTRIES }) })}
                status={READY}
                dirty={[]}
                onOpenProject={onOpenProject}
                against="base"
                onAgainst={onAgainst}
                git={gitWith(gitStatusOf())}
                unsaved={null}
                onGitSetup={onGitSetup}
                goTo={goTo}
                revision={0}
                chosen={chosen}
                onChoose={onChoose}
              />,
            ),
          ),
        );
      await show({ kind: "trainer", index: 1 });
      expect(change("Trainer 0001")?.getAttribute("aria-selected")).toBe("true");
      await act(async () => change("Matrix 0000")!.click());
      expect(onChoose).toHaveBeenLastCalledWith({ kind: "matrix", index: 0 });
      // An entity the list does not hold: the app is told to forget it.
      onChoose.mockClear();
      await show({ kind: "species", index: 25 });
      expect(onChoose).toHaveBeenCalledExactlyOnceWith(null);
    });
  });
});
