// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createTextArchive, dialogOf, dialogsOfHeader, editTextArchive, getTextArchive, searchText, validateText } from "../../api";
import type { Dialog } from "../../api";
import { TRAINER_ROWS } from "../../data/trainer/trainerTesting";
import { listToasts, resetToasts } from "../../ui/frame/toasts";
import { TextArea } from "./TextArea";
import { useTextArea } from "./TextAreaContext";
import { archive, figure, flush, headerDialogs, projectA, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  editTextArchive: vi.fn(),
  createTextArchive: vi.fn(),
  searchText: vi.fn(),
  validateText: vi.fn(),
  dialogsOfHeader: vi.fn(),
  dialogOf: vi.fn(),
  getMatrix: vi.fn(async (index: number) => ({ index, name: "m", width: 1, height: 1, maps: [7] })),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const TWINLEAF = archive(433, ["Nowhere", "Twinleaf Town", "Sandgem Town"]);

describe("TextArea", () => {
  let h: ReturnType<typeof textHarness>;

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset().mockImplementation(async (index) => (index === 433 ? TWINLEAF : archive(index, ["Neuer Text"])));
    vi.mocked(editTextArchive).mockReset().mockResolvedValue(1);
    vi.mocked(createTextArchive).mockReset().mockResolvedValue(435);
    vi.mocked(searchText).mockReset().mockResolvedValue([{ archive: 433, index: 1, excerpt: "…Twinleaf Town…" }]);
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    // jsdom has no layout, hence no scrollIntoView.
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => {
    h.unmount();
    vi.useRealTimers();
  });

  it("lists every archive, grouped into places and game-wide ones, without reading any archive", async () => {
    h.show(h.base());
    await flush();
    expect(h.container.querySelectorAll("[data-archive]")).toHaveLength(435);
    expect(h.archiveRow(433)?.getAttribute("aria-label")).toBe("Zweiblattdorf, Textarchiv 0433");
    expect(h.archiveRow(5)?.getAttribute("aria-label")).toBe("Sandgemme, Textarchiv 0005");
    expect(h.archiveRow(6)?.getAttribute("aria-label")).toBe("Textarchiv 0006");
    // The order of the tree: the group "Orte" with the archives a header uses, then "Spielweit" with all the others.
    const order = [...h.container.querySelectorAll("[data-text-group], [data-archive]")].map((e) => e.getAttribute("data-text-group") ?? e.getAttribute("data-archive"));
    expect(order.slice(0, 4)).toEqual(["orte", "5", "433", "spielweit"]);
    expect(order.slice(4, 6)).toEqual(["0", "1"]);
    expect(getTextArchive).not.toHaveBeenCalled();
  });

  it("loads the messages of a clicked archive and shows them with visible control characters", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Zeile\\nzwei\\rdrei\\f", "Eins"]));
    await h.open433();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(h.messageRow(0)?.textContent).toContain("Zeile⏎zwei▼drei▽");
    expect(h.messageRow(1)?.textContent).toContain("Eins");
    // The first message is selected; the field shows the text as stored.
    expect(h.textarea()?.value).toBe("Zeile\\nzwei\\rdrei\\f");
    // The count of the opened archive is known now.
    expect(h.container.textContent).toContain("2 Nachrichten");
    expect(h.archiveRow(433)?.getAttribute("aria-selected")).toBe("true");
  });

  it("shows a decode error in the hand and keeps the list working", async () => {
    vi.mocked(getTextArchive).mockImplementation(async (index) => {
      if (index === 7) throw { code: "error", message: "Textarchiv 0007 nicht lesbar", asset: null };
      return TWINLEAF;
    });
    h.show(h.base());
    await flush();
    h.click(h.archiveRow(7));
    await flush();
    expect(h.container.querySelector('[role="alert"]')?.textContent).toContain("Textarchiv 0007 nicht lesbar");
    expect(h.textarea()).toBeNull();
    expect(h.calls.onError).not.toHaveBeenCalled();
    h.click(h.archiveRow(433));
    await flush();
    expect(h.messageRow(1)?.textContent).toContain("Twinleaf Town");
    expect(h.container.textContent).not.toContain("nicht lesbar");
  });

  it("commits a typed text on blur as one Set edit and refreshes the dirty count", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    expect(h.textarea()?.value).toBe("Twinleaf Town");
    h.type(h.textarea()!, "Zweiblattdorf");
    expect(editTextArchive).not.toHaveBeenCalled();
    h.blur(h.textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledTimes(1);
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zweiblattdorf" } });
    expect(h.calls.onEdited).toHaveBeenCalled();
    // A blur without a change writes nothing.
    h.blur(h.textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledTimes(1);
  });

  it("tells the data areas which archive an edit or a new archive changed (names and descriptions live there)", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    h.type(h.textarea()!, "Zweiblattdorf");
    h.blur(h.textarea()!);
    await flush();
    expect(h.events).toContainEqual({ topic: "textData", archive: 433 });
    // A refused edit changed nothing.
    h.events.length = 0;
    vi.mocked(editTextArchive).mockRejectedValueOnce({ code: "error", message: "nein", asset: null });
    h.type(h.textarea()!, "Zweiblattdorf neu");
    h.blur(h.textarea()!);
    await flush();
    expect(h.events.filter((e) => e.topic === "textData")).toEqual([]);
    h.click(h.button("Textarchiv anlegen"));
    await flush();
    expect(h.events).toContainEqual({ topic: "textData", archive: 435 });
  });

  it("stores a typed or pasted line break as the escape the archive uses", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    h.type(h.textarea()!, "Zwei\nZeilen\r\nDrei");
    expect(h.textarea()?.value).toBe("Zwei\\nZeilen\\nDrei");
    h.blur(h.textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zwei\\nZeilen\\nDrei" } });
  });

  it("colours the control codes of the selected message in a preview below the field", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Hi {PLAYER}\\n[PK]"]));
    await h.open433();
    const kinds = [...h.container.querySelectorAll("[data-segment]")].map((e) => `${e.getAttribute("data-segment")}:${e.textContent}`);
    expect(kinds).toEqual(["text:Hi ", "code:{PLAYER}", "newline:⏎", "code:[PK]"]);
  });

  it("commits after 500 ms of rest, once", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    vi.useFakeTimers();
    h.type(h.textarea()!, "Zweiblatt");
    h.type(h.textarea()!, "Zweiblattdorf");
    await act(() => vi.advanceTimersByTimeAsync(400));
    expect(editTextArchive).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(200));
    expect(editTextArchive).toHaveBeenCalledTimes(1);
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zweiblattdorf" } });
    await act(() => vi.advanceTimersByTimeAsync(2000));
    expect(editTextArchive).toHaveBeenCalledTimes(1);
  });

  it("commits a pending text before it switches to another message", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    h.type(h.textarea()!, "Neu");
    h.click(h.messageRow(2));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Neu" } });
    expect(h.textarea()?.value).toBe("Sandgem Town");
  });

  it("reports a refused edit like any command error", async () => {
    const refused = { code: "error", message: "text archive 433 has 3 messages, none at 3", asset: null };
    vi.mocked(editTextArchive).mockRejectedValueOnce(refused);
    await h.open433();
    h.type(h.textarea()!, "x");
    h.blur(h.textarea()!);
    await flush();
    expect(h.calls.onError).toHaveBeenCalledWith(refused);
  });

  it("shows the stored text again when an edit is refused, also under StrictMode", async () => {
    vi.mocked(editTextArchive).mockRejectedValueOnce({ code: "error", message: "refused", asset: null });
    h.showStrict(h.base());
    await flush();
    h.click(h.archiveRow(433));
    await flush();
    h.click(h.messageRow(1));
    h.type(h.textarea()!, "Zweiblattdorf");
    h.blur(h.textarea()!);
    await flush();
    expect(h.calls.onError).toHaveBeenCalledTimes(1);
    expect(h.textarea()?.value).toBe("Twinleaf Town");
  });

  it("starts over when another project is opened and never writes a pending text into it", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    h.type(h.searchBox()!, "twin");
    h.type(h.textarea()!, "Text des alten Projekts");
    vi.useFakeTimers();
    // Another project: a new info object. The pending commit (500 ms) must be dropped, not flushed.
    h.show(h.base({ project: { ...projectA, root: "/b" } }));
    await act(() => vi.advanceTimersByTimeAsync(2000));
    vi.useRealTimers();
    await flush();
    expect(editTextArchive).not.toHaveBeenCalled();
    expect(h.textarea()).toBeNull();
    expect(h.container.querySelector("[data-message]")).toBeNull();
    expect(h.searchBox()?.value).toBe("");
    expect(h.container.querySelector("[data-hit]")).toBeNull();
    // The new project's archive reads fresh when chosen.
    vi.mocked(getTextArchive).mockClear();
    h.click(h.archiveRow(433));
    await flush();
    expect(getTextArchive).toHaveBeenCalledTimes(1);
    expect(h.textarea()?.value).toBe("Nowhere");
  });

  it("warns about a text that does not fit, from validateText", async () => {
    vi.mocked(validateText).mockResolvedValue([{ severity: "warning", file: "", line: 2, col: 1, message: "line is 260 px wide, the box holds 224 px", source: "text" }]);
    await h.open433();
    vi.useFakeTimers();
    h.type(h.textarea()!, "W".repeat(60));
    await act(() => vi.advanceTimersByTimeAsync(350));
    expect(validateText).toHaveBeenLastCalledWith("W".repeat(60));
    expect(h.container.textContent).toContain("Zeile 2: line is 260 px wide, the box holds 224 px");
  });

  it("marks exactly the selected message row as selected", async () => {
    await h.open433();
    h.click(h.messageRow(1));
    expect(h.messageRow(1)?.getAttribute("aria-selected")).toBe("true");
    expect(h.messageRow(0)?.getAttribute("aria-selected")).toBe("false");
    expect(h.messageRow(2)?.getAttribute("aria-selected")).toBe("false");
  });

  it("appends a message and selects it, and removes the last one", async () => {
    await h.open433();
    vi.mocked(getTextArchive).mockResolvedValue(archive(433, ["Nowhere", "Twinleaf Town", "Sandgem Town", ""]));
    h.click(h.button("Nachricht anlegen"));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, "Append");
    expect(h.messageRow(3)).not.toBeNull();
    expect(h.messageRow(3)?.getAttribute("aria-selected")).toBe("true");

    vi.mocked(getTextArchive).mockResolvedValue(TWINLEAF);
    h.click(h.button("Letzte Nachricht entfernen"));
    await flush();
    expect(editTextArchive).toHaveBeenLastCalledWith(433, "RemoveLast");
    expect(h.messageRow(3)).toBeNull();
    // The selection moved to a message that exists.
    expect(h.messageRow(2)?.getAttribute("aria-selected")).toBe("true");
  });

  it("searches all archives after 300 ms and selects the archive and message of a clicked hit", async () => {
    h.show(h.base());
    await flush();
    vi.useFakeTimers();
    h.type(h.searchBox()!, "twin");
    await act(() => vi.advanceTimersByTimeAsync(250));
    expect(searchText).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(100));
    expect(searchText).toHaveBeenCalledTimes(1);
    expect(vi.mocked(searchText).mock.calls[0][0]).toBe("twin");
    vi.useRealTimers();
    await flush();
    const hit = h.container.querySelector<HTMLElement>('[data-hit="433:1"]');
    // Archive and message in columns of their own, then the excerpt.
    expect([...hit!.querySelectorAll('[role="gridcell"]')].map((c) => c.textContent)).toEqual(["0433", "001", "…Twinleaf Town…"]);
    h.click(hit);
    await flush();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(h.container.querySelector('[data-hit="433:1"]')?.getAttribute("aria-selected")).toBe("true");
    expect(h.textarea()?.value).toBe("Twinleaf Town");
  });

  it("opens the message of a hit that Enter is pressed on and puts the focus into its field, not into the one open before", async () => {
    vi.mocked(searchText).mockResolvedValue([
      { archive: 433, index: 1, excerpt: "…Twinleaf Town…" },
      { archive: 5, index: 0, excerpt: "…Neuer Text…" },
    ]);
    await h.open433();
    h.type(h.searchBox()!, "text");
    await act(() => new Promise((resolve) => setTimeout(resolve, 350)));
    await flush();
    const hit = h.container.querySelector<HTMLElement>('[data-hit="5:0"]')!;
    expect(hit.getAttribute("aria-selected")).toBe("false");
    h.key(hit, { key: "Enter" });
    await flush();
    expect(getTextArchive).toHaveBeenCalledWith(5);
    expect(h.container.querySelector('[data-hit="5:0"]')?.getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(h.textarea());
    expect(h.textarea()?.value).toBe("Neuer Text");
  });

  it("searches only the open archive in the scope of this archive, without asking the backend", async () => {
    await h.open433();
    h.click([...h.container.querySelectorAll('[role="group"][aria-label="Suchbereich"] button')].find((b) => b.textContent === "Dieses Archiv"));
    h.type(h.searchBox()!, "town");
    await flush();
    expect([...h.container.querySelectorAll("[data-message]")].map((e) => e.getAttribute("data-message"))).toEqual(["1", "2"]);
    expect(h.container.querySelectorAll("[data-hit]")).toHaveLength(2);
    h.type(h.searchBox()!, "Pokécenter");
    expect(h.container.textContent).toContain("Nichts gefunden für „Pokécenter“");
    await act(() => new Promise((resolve) => setTimeout(resolve, 350)));
    expect(searchText).not.toHaveBeenCalled();
  });

  it("says when the search over all archives found nothing", async () => {
    vi.mocked(searchText).mockResolvedValue([]);
    h.show(h.base());
    await flush();
    h.type(h.searchBox()!, "xyz");
    await act(() => new Promise((resolve) => setTimeout(resolve, 350)));
    await flush();
    expect(h.container.textContent).toContain("Nichts gefunden für „xyz“");
  });

  it("filters the archive list by number or place without asking the backend", async () => {
    h.show(h.base());
    await flush();
    h.type(h.listFilter()!, "0433");
    expect([...h.container.querySelectorAll("[data-archive]")].map((e) => e.getAttribute("data-archive"))).toEqual(["433"]);
    h.type(h.listFilter()!, "Sandgem");
    expect([...h.container.querySelectorAll("[data-archive]")].map((e) => e.getAttribute("data-archive"))).toEqual(["5"]);
    expect(searchText).not.toHaveBeenCalled();
  });

  it("creates a new archive, refreshes the counts and selects it", async () => {
    h.show(h.base());
    await flush();
    h.click(h.button("Textarchiv anlegen"));
    await flush();
    expect(createTextArchive).toHaveBeenCalledTimes(1);
    expect(h.calls.onCountsChanged).toHaveBeenCalled();
    expect(getTextArchive).toHaveBeenCalledWith(435);
    expect(h.textarea()?.value).toBe("Neuer Text");
  });

  it("opens the archive and message a link led to", async () => {
    h.show(h.base({ target: { index: 433, message: 2 } }));
    await flush();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(h.messageRow(2)?.getAttribute("aria-selected")).toBe("true");
    expect(h.textarea()?.value).toBe("Sandgem Town");
    expect(h.calls.onShown).toHaveBeenLastCalledWith({ archive: 433, message: 2 });
  });

  it("reads the open archive again when undo or redo moved it (revision), and not for the same context", async () => {
    await h.open433();
    const calls = vi.mocked(getTextArchive).mock.calls.length;
    h.show(h.base());
    await flush();
    expect(vi.mocked(getTextArchive).mock.calls.length).toBe(calls);
    vi.mocked(getTextArchive).mockResolvedValue(archive(433, ["Nowhere", "Zurück", "Sandgem Town"]));
    h.show(h.base({ revision: 1 }));
    await flush();
    expect(h.messageRow(1)?.textContent).toContain("Zurück");
    expect(h.textarea()?.value).toBe("Nowhere");
  });

  it("links the headers that use the open archive", async () => {
    await h.open433();
    h.click(h.container.querySelector('[data-header="12"]'));
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "header", id: 12 });
    expect(h.container.querySelector('[data-header="13"]')).not.toBeNull();
  });

  it("counts the overlong texts per archive from the problems store, with a sum below the list", async () => {
    h.show(h.base());
    await flush();
    expect(h.container.textContent).not.toContain("zu lang, in");
    const warning = (archiveIndex: number, line: number) => ({ severity: "warning" as const, file: `expanded/textArchives/${String(archiveIndex).padStart(4, "0")}.json`, line, col: 1, message: "Zeile zu lang", source: "text" as const });
    act(() => h.problems.replaceText(433, [warning(433, 1), warning(433, 4)]));
    act(() => h.problems.replaceText(5, [warning(5, 2)]));
    expect(h.archiveRow(433)?.textContent).toContain("2");
    expect(h.archiveRow(5)?.textContent).toContain("1");
    expect(h.container.textContent).toContain("3 Texte zu lang, in 2 geprüften Archiven");
  });

  it("names the open archive, its messages and the overlong ones in the status, and the keys while the field has focus", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Kurz", "x".repeat(45), "Auch kurz"]));
    await h.open433();
    const status = h.container.querySelector("[data-status]")!;
    expect(status.textContent).toBe("Archiv 0433, 3 Nachrichten, 1 zu lang");
    const hints = h.container.querySelector("[data-hints]")!;
    expect(hints.textContent).toBe("");
    h.focus(h.textarea()!);
    expect(hints.textContent).toBe("↵ neue Zeile, ⇧↵ neue Seite");
    h.blur(h.textarea()!);
    expect(hints.textContent).toBe("");
  });

  it("jumps into the editor with Enter on a message row", async () => {
    await h.open433();
    const row = h.messageRow(2)!;
    h.click(row);
    h.key(row, { key: "Enter" });
    expect(document.activeElement).toBe(h.textarea());
    expect(h.textarea()?.value).toBe("Sandgem Town");
  });

  it("pages through the selected message in the view", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Eins\\nZwei\\rDrei\\fVier"]));
    await h.open433();
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 1 von 3");
    h.click(h.button("Weiterblättern"));
    expect(h.screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 2 von 3");
    expect(h.calls.onShown).toHaveBeenLastCalledWith({ archive: 433, message: 0 });
  });

  it("shows what is missing: no project, no archive", async () => {
    h.show(h.base({ project: null, counts: {} }));
    await flush();
    expect(h.container.textContent).toContain("Kein Projekt geöffnet.");
    h.show(h.base());
    await flush();
    expect(h.container.textContent).toContain("Wähle links ein Archiv.");
    expect(h.textarea()).toBeNull();
  });

  it("narrates while it shows Erzählen", () => {
    let narrating: boolean | undefined;
    function Probe() {
      narrating = useTextArea().narrating;
      return null;
    }
    const narratingIn = (depth: "erzaehlen" | "bauen" | "tuefteln") => {
      act(() =>
        h.root.render(
          <TextArea context={h.base({ depth })}>
            <Probe />
          </TextArea>,
        ),
      );
      return narrating;
    };
    expect(narratingIn("erzaehlen")).toBe(true);
    expect(narratingIn("bauen")).toBe(false);
    expect(narratingIn("tuefteln")).toBe(false);
  });

  describe("dialogs by place and figure (Erzählen)", () => {
    const said = (texts: string[]): Dialog => ({
      source: { kind: "local", file: 40, slot: 2, line: 1 },
      messages: texts.map((text, i) => ({ archive: 433, index: 4 + i, text, command: "Message", line: 3 + i, col: 13, branch: null, shared_with: i === 1 ? 2 : 0, split_refusal: null })),
      followed: [],
    });
    const FIGURES = [
      figure("object", 3, { first_text: "Schönes Wetter!", messages: 2, pages: 2, message_ids: [4, 5] }),
      figure("object", 5, { source: { kind: "trainer", id: 12 }, trainer: 12, first_text: null, messages: 0, pages: 0, message_ids: [] }),
      figure("bg", 0, { first_text: "Zweiblattdorf", message_ids: [1] }),
    ];
    const card = (key: string) => h.container.querySelector<HTMLElement>(`[data-dialog-figure="${key}"]`);
    const place = (name: string) => h.container.querySelector<HTMLElement>(`[data-dialog-place="${name}"]`);
    const headerRow = (id: number) => h.container.querySelector<HTMLElement>(`[data-dialog-header="${id}"]`);
    const within = (el: Element | null, name: string) => [...(el?.querySelectorAll("button") ?? [])].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
    const hand = () => h.container.querySelector<HTMLElement>("[data-dialog-hand]");
    async function openHeader(id: number, placeName = "Zweiblattdorf") {
      h.click(place(placeName));
      await flush();
      h.click(headerRow(id));
      await flush();
    }

    beforeEach(() => {
      resetToasts();
      vi.mocked(dialogsOfHeader).mockReset().mockImplementation(async (id) => headerDialogs(id, id === 12 ? FIGURES : id === 13 ? [figure("object", 0, { message_ids: [2] })] : []));
      vi.mocked(dialogOf).mockReset().mockResolvedValue(said(["Schönes Wetter!", "Tschüss."]));
    });

    it("in erzaehlen the list shows places and the view the figures", async () => {
      h.show(h.base({ depth: "erzaehlen", trainers: TRAINER_ROWS }));
      await flush();
      expect(h.container.querySelector("[data-dialog-list]")).not.toBeNull();
      expect(h.container.querySelector("[data-archive]")).toBeNull();
      expect(place("Zweiblattdorf")).not.toBeNull();
      await openHeader(12);
      // The object with its own script, the trainer and the sign: all three speak.
      expect(headerRow(12)?.textContent).toContain("3 Figuren sprechen");
      const view = h.container.querySelector("[data-dialog-cards]");
      expect([...view!.querySelectorAll("[data-dialog-figure]")].map((e) => e.getAttribute("data-dialog-figure"))).toEqual(["object:3", "object:5", "bg:0"]);
      expect(card("object:3")?.textContent).toContain("2 Nachrichten · 2 Seiten");
      // The archives' table is gone from the hand: it asks for a figure.
      expect(h.messageRow(0)).toBeNull();
      expect(hand()?.textContent).toContain("Wähle eine Figur.");
      // Bauen shows the archives again.
      h.show(h.base({ depth: "bauen" }));
      await flush();
      expect(h.container.querySelector("[data-dialog-list]")).toBeNull();
      expect(h.archiveRow(433)).not.toBeNull();
    });

    it("a click selects the figure and the hand shows its messages", async () => {
      h.show(h.base({ depth: "erzaehlen" }));
      await flush();
      await openHeader(12);
      h.click(within(card("object:3"), "Objekt 3"));
      await flush();
      expect(card("object:3")?.hasAttribute("data-selected")).toBe(true);
      expect(card("bg:0")?.hasAttribute("data-selected")).toBe(false);
      expect(h.container.querySelector("[data-status]")?.textContent).toBe("Zweiblattdorf, 3 Figuren sprechen, Objekt 3");
      expect(dialogOf).toHaveBeenCalledWith(12, 112, "object", 3);
      // The messages of the dialog editor: a field per message, the shared one with its badge and the split.
      const blocks = hand()!.querySelectorAll("[data-dialog-message]");
      expect([...blocks].map((b) => b.getAttribute("data-dialog-message"))).toEqual(["433:4", "433:5"]);
      expect(blocks[0]!.querySelector("textarea")?.value).toBe("Schönes Wetter!");
      expect(blocks[1]!.textContent).toContain("Geteilt mit 2 Figuren");
      expect(within(blocks[1]!, "Für diese Figur abspalten")).toBeDefined();
      // A text written in the hand goes to the store as one step „Dialog geändert“ and tells the app.
      const field = blocks[0]!.querySelector("textarea")!;
      h.type(field, "Regen!");
      h.blur(field);
      await flush();
      expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 4, text: "Regen!" } }, "Dialog geändert");
      expect(h.calls.onEdited).toHaveBeenCalled();
      expect(h.events).toContainEqual({ topic: "textData", archive: null });
    });

    it("the edit button opens the dialog editor", async () => {
      h.show(h.base({ depth: "erzaehlen" }));
      await flush();
      await openHeader(12);
      h.click(within(card("object:3"), "Dialog bearbeiten…"));
      expect(h.calls.onOpenDialog).toHaveBeenLastCalledWith({ header: 12, eventFile: 112, kind: "object", index: 3, title: "Objekt 3" });
      // A double click on a card does the same.
      act(() => void within(card("bg:0"), "Schild 0")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
      expect(h.calls.onOpenDialog).toHaveBeenLastCalledWith({ header: 12, eventFile: 112, kind: "bg", index: 0, title: "Schild 0" });
      // The hand of a chosen figure has the way in too.
      h.click(within(card("object:3"), "Objekt 3"));
      await flush();
      h.calls.onOpenDialog.mockClear();
      h.click(within(hand(), "Dialog bearbeiten…"));
      expect(h.calls.onOpenDialog).toHaveBeenCalledWith({ header: 12, eventFile: 112, kind: "object", index: 3, title: "Objekt 3" });
    });

    it("a trainer card opens the trainer, not the editor", async () => {
      h.show(h.base({ depth: "erzaehlen", trainers: TRAINER_ROWS }));
      await flush();
      await openHeader(12);
      const trainer = card("object:5")!;
      expect(trainer.textContent).toContain("Trainer Wanderer Jens");
      expect(trainer.textContent).toContain("Kampftexte des Trainers: im Bereich Pokémon");
      expect(within(trainer, "Dialog bearbeiten…")).toBeUndefined();
      h.click(within(trainer, "Trainer öffnen"));
      expect(h.calls.goTo).toHaveBeenLastCalledWith({ kind: "trainer", index: 12 });
      act(() => void within(trainer, "Trainer Wanderer Jens")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
      expect(h.calls.goTo).toHaveBeenCalledTimes(2);
      // Chosen, the hand says where its texts are and leads there; nothing reads its dialog.
      h.click(within(trainer, "Trainer Wanderer Jens"));
      await flush();
      expect(hand()?.textContent).toContain("Kampftexte des Trainers: im Bereich Pokémon");
      expect(hand()?.querySelector("[data-dialog-message]")).toBeNull();
      h.click(within(hand(), "Trainer öffnen"));
      expect(h.calls.goTo).toHaveBeenCalledTimes(3);
      expect(h.calls.onOpenDialog).not.toHaveBeenCalled();
      expect(dialogOf).not.toHaveBeenCalled();
    });

    it("a place without figures says so", async () => {
      h.show(h.base({ depth: "erzaehlen" }));
      await flush();
      await openHeader(14, "Sandgemme");
      expect(headerRow(14)?.textContent).toContain("Keine Figuren");
      expect(h.container.querySelector("[data-dialog-cards]")?.textContent).toContain("Keine Figuren an diesem Ort");
      expect(h.container.querySelector("[data-dialog-figure]")).toBeNull();
    });

    it("a link to a message selects the place and the figure that says it", async () => {
      h.show(h.base({ depth: "erzaehlen" }));
      await flush();
      h.show(h.base({ depth: "erzaehlen", target: { index: 433, message: 2 } }));
      await flush();
      await flush();
      // Header 12 (first of the archive) has no figure that says message 2; header 13's object 0 does.
      expect(dialogsOfHeader).toHaveBeenCalledWith(12);
      expect(dialogsOfHeader).toHaveBeenCalledWith(13);
      expect(place("Zweiblattdorf")?.getAttribute("aria-expanded")).toBe("true");
      expect(headerRow(13)?.getAttribute("aria-selected")).toBe("true");
      expect(card("object:0")?.hasAttribute("data-selected")).toBe(true);
      expect(h.calls.onDepth).not.toHaveBeenCalled();
    });

    it("a link without a message selects the first place of the archive", async () => {
      h.show(h.base({ depth: "erzaehlen", target: { index: 433, message: null } }));
      await flush();
      expect(headerRow(12)?.getAttribute("aria-selected")).toBe("true");
      expect(h.container.querySelector("[data-dialog-figure][data-selected]")).toBeNull();
      expect(h.calls.onDepth).not.toHaveBeenCalled();
    });

    it("a link to a message no figure says opens Bauen and says why", async () => {
      h.show(h.base({ depth: "erzaehlen", target: { index: 433, message: 7 } }));
      await flush();
      await flush();
      expect(h.calls.onDepth).toHaveBeenCalledWith("bauen");
      expect(listToasts().map((t) => [t.kind, t.text])).toContainEqual(["info", "Keine Figur spricht diese Nachricht – in Bauen geöffnet"]);
      // Bauen then shows the archive and message the link asked for.
      h.show(h.base({ depth: "bauen", target: { index: 433, message: 7 } }));
      await flush();
      expect(h.calls.onShown).toHaveBeenLastCalledWith({ archive: 433, message: 2 });
    });
  });
});
