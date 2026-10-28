// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { conflictDiff, resolveExternalConflict } from "../api";
import type { ConflictDiff } from "../api";
import { TextProjectSurface } from "./TextProjectSurface";
import type { TextProjectView } from "./TextProjectSurface";
import { conflict, outcome, syncWith, textWith } from "./textProjectTesting";

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
    expect(status).toContain("Gleiche mit DSPRE ab … 120 von 6500 Dateien");
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
    expect(row("moves/0001")?.textContent).toContain("und 3 weitere Abweichungen");
    expect(row("moves/0001")?.textContent).not.toContain("verwirft");
    expect(button("Studio-Fassung behalten", row("moves/0001")!)?.getAttribute("data-kind")).toBe("secondary");
    expect(button("DSPRE-Fassung übernehmen", row("moves/0001")!)?.getAttribute("data-kind")).toBe("secondary");
    await click(button("Studio-Fassung behalten", row("moves/0001")!));
    expect(resolveExternalConflict).toHaveBeenCalledExactlyOnceWith("moves/0001", "studio");
    expect(onResolved).toHaveBeenCalledExactlyOnceWith([{ kind: "Move", index: 1 }]);

    // A source changed on disk: the other side is the file.
    vi.mocked(conflictDiff).mockResolvedValue({ lines: [{ path: "name", studio: '"Pikachu"', dspre: '"Pika"' }], more: 0 });
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
});
