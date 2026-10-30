// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { diffEntity } from "../api";
import type { Against, ChangedEntry, EntityDiffReport } from "../api";
import { ResetProvider } from "../original/OriginalContext";
import { MISSING, readyWith, resetSpy } from "../original/resetTesting";
import { entry } from "./changesTesting";
import { diffReport } from "./diffTesting";
import { EntityDiff } from "./EntityDiff";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  diffEntity: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const TRAINER = entry({ group: "Trainer", label: "Trainer 0001", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], folder: "trainer_properties", subject: { kind: "trainer", index: 1 } });

describe("EntityDiff", () => {
  let container: HTMLElement;
  let root: Root;
  let reset: ReturnType<typeof resetSpy>;
  const goTo = vi.fn();

  beforeEach(() => {
    goTo.mockClear();
    reset = resetSpy();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** The detail of `row` against `against`, the kernel answering `answer`; a Platinum project (the header fields' names). */
  const render = async (answer: EntityDiffReport | null, { row = TRAINER, against = "head" }: { row?: ChangedEntry | null; against?: Against } = {}) => {
    if (answer !== null) vi.mocked(diffEntity).mockReset().mockResolvedValue(answer);
    await act(async () =>
      root.render(
        <ResetProvider value={{ status: reset.status, open: reset.open, family: "Pt" }}>
          <EntityDiff entry={row} against={against} revision={0} goTo={goTo} />
        </ResetProvider>,
      ),
    );
  };
  const detail = () => container.querySelector<HTMLElement>("[data-entity-diff]");
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const tables = () => [...container.querySelectorAll("table")];
  const head = (table: HTMLTableElement | undefined) => [...(table?.querySelectorAll("thead th") ?? [])].map((th) => th.textContent);
  const body = (table: HTMLTableElement | undefined) => [...(table?.querySelectorAll("tbody tr") ?? [])].map((tr) => [...tr.querySelectorAll("th, td")].map((c) => c.textContent));
  const reason = (b: HTMLElement | undefined) => document.getElementById(b?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("fields_show_path_old_and_new_with_side_words", async () => {
    // Nothing chosen: what to do.
    await render(null, { row: null });
    expect(detail()?.textContent).toBe("Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.");
    // Chosen: loading, then the head and the fields.
    let answer: (r: EntityDiffReport) => void = () => {};
    vi.mocked(diffEntity).mockReset().mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
    await act(async () =>
      root.render(
        <ResetProvider value={{ status: reset.status, open: reset.open }}>
          <EntityDiff entry={TRAINER} against="head" revision={0} goTo={goTo} />
        </ResetProvider>,
      ),
    );
    expect(detail()?.querySelector("[role=status]")?.textContent).toBe("Lade Vergleich …");
    await act(async () => answer(diffReport({ diff: { shape: "fields", changes: [{ path: "data.party[0].level", old: "5", new: "6" }, { path: "data.items[1]", old: null, new: "HP_UP" }], more: 4 } })));
    expect(diffEntity).toHaveBeenCalledWith({ kind: "trainer", index: 1 }, "head");
    expect(detail()?.querySelector("h2")?.textContent).toBe("Trainer 0001");
    expect(detail()?.textContent).toContain("Commit 4c1d2e3 „Ausgangsstand“ → Projekt (gespeichert)");
    expect(head(tables()[0])).toEqual(["Feld", "Commit", "Projekt"]);
    expect(body(tables()[0])).toEqual([
      ["data.party[0].level", "5", "6"],
      ["data.items[1]", "—fehlt", "HP_UP"],
    ]);
    expect(detail()?.textContent).toContain("und 4 weitere Felder");
    // Against the base and the saved state: their side words.
    await render(diffReport({ against: "base", old_label: "Basis-ROM Platin DE" }), { against: "base" });
    expect(head(tables()[0])).toEqual(["Feld", "Basis", "Projekt"]);
    await render(diffReport({ against: "saved", old_label: "Gespeichert", new_label: "Ungespeichert" }), { against: "saved" });
    expect(head(tables()[0])).toEqual(["Feld", "Gespeichert", "Ungespeichert"]);
    // Equal after all: says so.
    await render(diffReport({ state: "same", diff: { shape: "fields", changes: [], more: 0 } }), { against: "base" });
    expect(detail()?.textContent).toContain("Keine Abweichung.");
    expect(tables()).toHaveLength(0);
    // Refused: the sentence.
    vi.mocked(diffEntity).mockReset().mockRejectedValue({ code: "no_commit", message: "Noch kein Commit: es gibt nichts zu vergleichen.", asset: null });
    await render(null, { against: "saved" });
    expect(detail()?.querySelector("[role=alert]")?.textContent).toContain("Noch kein Commit: es gibt nichts zu vergleichen.");
  });

  it("header_fields_show_their_german_name", async () => {
    const header = entry({ group: "Header", label: "Header 0003", target: { kind: "header", id: 3 }, assets: [{ kind: "HeaderTable", index: 0 }], folder: null, subject: { kind: "header", index: 3 } });
    await render(
      diffReport({
        subject: { kind: "header", index: 3 },
        label: "Header 0003",
        reset: { kind: "header", id: 3 },
        diff: { shape: "fields", changes: [{ path: "weather_id", old: "0", new: "3" }, { path: "unknown_field", old: "1", new: "2" }], more: 0 },
      }),
      { row: header },
    );
    expect(body(tables()[0])).toEqual([
      ["Wetter", "0", "3"],
      ["unknown_field", "1", "2"],
    ]);
    // The path stays at hand as the tooltip.
    expect(tables()[0]?.querySelector("tbody th [title]")?.getAttribute("title")).toBe("weather_id");
  });

  it("map_shows_parts_cells_and_buildings", async () => {
    const map = entry();
    await render(
      diffReport({
        subject: { kind: "map", index: 3 },
        label: "Karte 0003",
        reset: { kind: "map", index: 3 },
        diff: {
          shape: "map",
          parts: [
            { part: "model", changed: false, old: "aaaaaaaa", new: "aaaaaaaa" },
            { part: "permissions", changed: true, old: "bbbbbbbb", new: "cccccccc" },
            { part: "buildings", changed: true, old: "dddddddd", new: "eeeeeeee" },
            { part: "bdhc", changed: false, old: "ffffffff", new: "ffffffff" },
            { part: "bgs", changed: false, old: null, new: null },
            { part: "raw", changed: false, old: null, new: null },
          ],
          cells: [
            { x: 4, y: 7, old: "0000", new: "0080" },
            { x: 5, y: 7, old: "", new: "0001" },
          ],
          buildings: [{ path: "buildings[0].x", old: "4", new: "5" }],
          more: 5,
        },
      }),
      { row: map },
    );
    const parts = detail()!.querySelector<HTMLElement>("[data-map-parts]")!;
    const partRows = [...parts.querySelectorAll("li")].map((li) => li.textContent);
    expect(partRows).toEqual(["Modellgleich", "Bewegungsrechtegeändert", "Gebäudegeändert", "BDHCgleich", "BGSgleich", "Rohdatengleich"]);
    expect(parts.querySelector("[class*=warning]")?.textContent).toBe("geändert");
    const titles = [...detail()!.querySelectorAll("h3")].map((h) => h.textContent);
    expect(titles.some((t) => t?.startsWith("Teile"))).toBe(true);
    expect(titles.some((t) => t?.startsWith("Zellen der Bewegungsrechte"))).toBe(true);
    expect(titles.some((t) => t?.startsWith("Gebäude"))).toBe(true);
    expect(head(tables()[0])).toEqual(["Zelle", "Commit", "Projekt"]);
    expect(body(tables()[0])).toEqual([
      ["4, 7", "0000", "0080"],
      ["5, 7", "—fehlt", "0001"],
    ]);
    expect(head(tables()[1])).toEqual(["Feld", "Commit", "Projekt"]);
    expect(body(tables()[1])).toEqual([["buildings[0].x", "4", "5"]]);
    // Cut rows count cells and building fields together: one line below the map, not in a table.
    expect(detail()?.querySelector("table [data-change-table-more]")).toBeNull();
    expect(detail()?.lastElementChild?.textContent).toBe("und 5 weitere Einträge");
    // Only building fields cut, no cell changed: the line is there all the same.
    await render(
      diffReport({ subject: { kind: "map", index: 4 }, label: "Karte 0004", diff: { shape: "map", parts: [], cells: [], buildings: [{ path: "buildings[0].x", old: "4", new: "5" }], more: 2 } }),
      { row: { ...map, subject: { kind: "map", index: 4 } } },
    );
    expect(detail()?.textContent).toContain("und 2 weitere Einträge");
  });

  it("messages_show_word_spans", async () => {
    const archive = entry({ group: "Texte", label: "Textarchiv 0433", target: { kind: "text_archive", index: 433 }, assets: [{ kind: "TextArchive", index: 433 }], folder: "textArchives", subject: { kind: "text_archive", index: 433 } });
    await render(
      diffReport({
        subject: { kind: "text_archive", index: 433 },
        label: "Textarchiv 0433",
        reset: { kind: "text_archive", index: 433 },
        diff: {
          shape: "messages",
          messages: [
            { index: 2, state: "changed", old: [{ op: "same", text: "Hallo " }, { op: "removed", text: "Welt" }], new: [{ op: "same", text: "Hallo " }, { op: "added", text: "Sinnoh" }] },
            { index: 9, state: "new", old: [], new: [{ op: "added", text: "Neu hier" }] },
            { index: 10, state: "removed", old: [{ op: "removed", text: "Weg" }], new: [] },
          ],
          more: 2,
        },
      }),
      { row: archive },
    );
    const table = tables()[0];
    expect(head(table)).toEqual(["Nachricht", "Commit", "Projekt"]);
    expect(body(table)).toEqual([
      ["2", "Hallo Welt", "Hallo Sinnoh"],
      ["9Neu", "—fehlt", "Neu hier"],
      ["10Entfernt", "Weg", "—fehlt"],
    ]);
    expect(table?.querySelector("tbody tr del")?.textContent).toBe("Welt");
    expect(table?.querySelector("tbody tr ins")?.textContent).toBe("Sinnoh");
    expect(detail()?.textContent).toContain("und 2 weitere Nachrichten");
  });

  it("lines_show_hunks", async () => {
    const script = entry({ group: "Scripts", label: "Script 0002", target: { kind: "script", file: 2 }, assets: [{ kind: "Script", index: 2 }], folder: "scripts", subject: { kind: "script", index: 2 } });
    await render(
      diffReport({
        subject: { kind: "script", index: 2 },
        label: "Script 0002",
        reset: { kind: "script", file: 2 },
        diff: {
          shape: "lines",
          hunks: [
            {
              old_start: 10,
              new_start: 12,
              lines: [
                ["same", "script First #1:"],
                ["removed", "    Message 1"],
                ["added", "    Message 2"],
              ],
            },
          ],
          more: 7,
        },
      }),
      { row: script },
    );
    const hunk = detail()!.querySelector<HTMLElement>("[data-hunk]")!;
    expect(hunk.querySelector("h3")?.textContent).toBe("Zeile 10 → 12");
    const lines = [...hunk.querySelectorAll("[data-line]")].map((l) => l.textContent);
    expect(lines).toEqual([" script First #1:", "−    Message 1", "+    Message 2"]);
    expect(hunk.querySelector("del")?.textContent).toBe("−    Message 1");
    expect(hunk.querySelector("ins")?.textContent).toBe("+    Message 2");
    expect(hunk.querySelector("code")).not.toBeNull();
    expect(detail()?.textContent).toContain("und 7 weitere Zeilen");
  });

  it("binary_shows_size_and_sha1", async () => {
    const texture = entry({ group: "Texturen", label: "Kartentextur 0001", target: { kind: "map_texture", index: 1 }, assets: [], folder: "mapTextures", subject: { kind: "map_texture", index: 1 } });
    await render(
      diffReport({
        subject: { kind: "map_texture", index: 1 },
        label: "Kartentextur 0001",
        reset: { kind: "map_texture", index: 1 },
        diff: { shape: "binary", old: { side: "base" }, new: { side: "bytes", size: 12345, sha1: "0123456789abcdef0123456789abcdef01234567" } },
      }),
      { row: texture },
    );
    expect(head(tables()[0])).toEqual(["Eigenschaft", "Commit", "Projekt"]);
    expect(body(tables()[0])).toEqual([
      ["Größe", "wie in der Basis-ROM", "12.345 Byte"],
      ["SHA-1", "wie in der Basis-ROM", "01234567"],
    ]);
    // Another texture, deleted since the commit.
    await render(diffReport({ diff: { shape: "binary", old: { side: "bytes", size: 4, sha1: "f".repeat(40) }, new: { side: "absent" } } }), { row: { ...texture, subject: { kind: "map_texture", index: 2 } } });
    expect(body(tables()[0])).toEqual([
      ["Größe", "4 Byte", "—fehlt"],
      ["SHA-1", "ffffffff", "—fehlt"],
    ]);
  });

  it("open_jumps_and_is_locked_without_a_place", async () => {
    await render(diffReport());
    const open = button("Öffnen")!;
    expect(open.getAttribute("data-kind")).toBe("secondary");
    await act(async () => open.click());
    expect(goTo).toHaveBeenCalledExactlyOnceWith({ kind: "trainer", index: 1 });
    // A map has no place: locked, with the reason.
    goTo.mockClear();
    await render(diffReport({ subject: { kind: "map", index: 3 }, label: "Karte 0003", reset: { kind: "map", index: 3 }, diff: { shape: "fields", changes: [], more: 0 } }), { row: entry() });
    const locked = button("Öffnen")!;
    expect(locked.getAttribute("aria-disabled")).toBe("true");
    expect(reason(locked)).toBe("Für diese Art gibt es keinen Ort in der App.");
    await act(async () => locked.click());
    expect(goTo).not.toHaveBeenCalled();
    // An event file whose diff is still read: locked without a reason (the place follows from the diff).
    vi.mocked(diffEntity).mockReset().mockImplementationOnce(() => new Promise(() => {}));
    const events = entry({ group: "Ereignisse", label: "Ereignisdatei 0005", target: { kind: "event_file", index: 5 }, assets: [], subject: { kind: "event_file", index: 5 } });
    await render(null, { row: events });
    const waiting = button("Öffnen")!;
    expect(waiting.getAttribute("aria-disabled")).toBe("true");
    expect(waiting.hasAttribute("aria-describedby")).toBe(false);
    expect(detail()?.textContent).not.toContain("Für diese Art gibt es keinen Ort in der App.");
    // No primary button in the detail.
    expect(container.querySelectorAll("[data-kind=primary]")).toHaveLength(0);
  });

  it("reset_opens_the_dialog_and_is_locked_for_a_new_entity", async () => {
    await render(diffReport());
    const resetButton = button("Auf Original zurücksetzen…")!;
    expect(resetButton.getAttribute("data-kind")).toBe("secondary");
    await act(async () => resetButton.click());
    expect(reset.open).toHaveBeenCalledExactlyOnceWith({ kind: "trainer", index: 1 }, expect.anything());
    // New since the commit: no original.
    reset.open.mockClear();
    await render(diffReport({ state: "new", diff: { shape: "fields", changes: [{ path: "data.class", old: null, new: "3" }], more: 0 } }), { row: entry({ ...TRAINER, state: "new" }) });
    const locked = button("Auf Original zurücksetzen…")!;
    expect(locked.getAttribute("aria-disabled")).toBe("true");
    expect(reason(locked)).toBe("Kein Original");
    await act(async () => locked.click());
    expect(reset.open).not.toHaveBeenCalled();
    // A kind without a reset: as on its row.
    await render(diffReport({ subject: { kind: "matrix", index: 0 }, label: "Matrix 0000", reset: null }), { row: entry({ group: "Matrizen", label: "Matrix 0000", target: null, assets: [], subject: { kind: "matrix", index: 0 } }) });
    expect(reason(button("Auf Original zurücksetzen…"))).toBe("Für diese Art gibt es kein Zurücksetzen.");
    // Against the last commit without a base ROM, and past the base's entries: as the entry of a menu says.
    reset = resetSpy(MISSING);
    await render(diffReport({ subject: { kind: "trainer", index: 2 } }), { row: { ...TRAINER, subject: { kind: "trainer", index: 2 } } });
    expect(reason(button("Auf Original zurücksetzen…"))).toBe("Basis-ROM fehlt");
    reset = resetSpy(readyWith({ trainer: 1 }));
    await render(diffReport({ subject: { kind: "trainer", index: 3 } }), { row: { ...TRAINER, subject: { kind: "trainer", index: 3 } } });
    expect(reason(button("Auf Original zurücksetzen…"))).toBe("Kein Original");
  });
});
