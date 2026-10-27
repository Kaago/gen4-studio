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
    expect(resetButton("Karte 0003")?.disabled).toBe(false);
    expect(resetButton("Karte 0003")?.getAttribute("data-kind")).toBe("secondary");
    expect(resetButton("Karte 0700")?.disabled).toBe(true);
    expect(resetButton("Karte 0700")?.title).toBe("Kein Original");
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
    expect(resetButton("Karte 0599")?.title).toBe("Kein Original");

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
    expect(locked.disabled).toBe(true);
    expect(locked.title).toBe("Für diese Art gibt es kein Zurücksetzen.");
    expect(badges(change("Matrix 0000"))).toEqual(["Geändert"]);
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
    expect(reset.open).toHaveBeenCalledExactlyOnceWith({ kind: "trainer", index: 1 });
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
