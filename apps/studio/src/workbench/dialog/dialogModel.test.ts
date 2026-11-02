import { describe, expect, it } from "vitest";
import type { Dialog, DialogMessage, FigureDialog } from "../../api";
import { dialogSummary, distinctMessages, figureKey, figureSummary, figureTarget, pageCount, sharedText, speakersText } from "./dialogModel";

const message = (index: number, text: string | null, over: Partial<DialogMessage> = {}): DialogMessage => ({ archive: 554, index, text, command: "Message", line: 3, col: 13, branch: null, shared_with: 0, split_refusal: null, ...over });
const local = (messages: DialogMessage[]): Dialog => ({ source: { kind: "local", file: 12, slot: 2, line: 3 }, messages, followed: [] });

describe("dialogModel", () => {
  it("summary_counts_messages_and_pages_and_words_the_other_sources", () => {
    // Two pages (a clear code in the middle) and one: three pages over two messages.
    const two = [message(4, String.raw`Hallo!\rWie geht's?`), message(5, "Tschüss.")];
    expect(pageCount(two)).toBe(3);
    expect(dialogSummary(local(two))).toBe("2 Nachrichten · 3 Seiten");
    expect(dialogSummary(local([message(4, "Hallo!")]))).toBe("1 Nachricht · 1 Seite");
    // A message the archive does not have counts as a message without a page.
    expect(pageCount([message(4, null)])).toBe(0);
    expect(dialogSummary(local([]))).toBe("Sagt nichts");
    expect(dialogSummary({ source: { kind: "none" }, messages: [], followed: [] })).toBe("Sagt nichts");
    expect(dialogSummary({ source: { kind: "common", script: 2004 }, messages: [], followed: [] })).toBe("Gemeinsames Script 2004: in Bauen bearbeiten.");
    expect(dialogSummary({ source: { kind: "trainer", id: 12 }, messages: [], followed: [] })).toBe("Kampftexte des Trainers: im Bereich Pokémon.");
    expect(dialogSummary({ source: { kind: "unreadable", reason: "Scripts nicht übernommen" }, messages: [], followed: [] })).toBe("Script nicht lesbar: Scripts nicht übernommen");
  });

  it("a_message_the_dialog_shows_twice_counts_once", () => {
    // `dialog_of` lists every use: message 5 on line 3 and again on line 9 (in a branch).
    const twice = [message(5, String.raw`Hallo!\rWie geht's?`), message(6, "Tschüss."), message(5, String.raw`Hallo!\rWie geht's?`, { line: 9, branch: "CheckFlag 100" })];
    const distinct = distinctMessages(twice);
    expect(distinct.map((d) => [d.message.index, d.message.line, d.others.map((o) => o.line)])).toEqual([
      [5, 3, [9]],
      [6, 3, []],
    ]);
    expect(dialogSummary(local(twice))).toBe("2 Nachrichten · 3 Seiten");
  });

  it("shared_text_singular_and_plural", () => {
    expect(sharedText(1)).toBe("Geteilt mit 1 Figur");
    expect(sharedText(3)).toBe("Geteilt mit 3 Figuren");
  });

  it("a_place_counts_the_figures_that_speak_and_a_card_says_what_its_figure_says", () => {
    const f = (source: FigureDialog["source"], messages = 1, pages = 1): FigureDialog => ({ kind: "object", index: 3, local_id: 3, graphics_id: 1, trainer: null, source, first_text: null, messages, pages, message_ids: [] });
    const local = { kind: "local", file: 1, slot: 1, line: 1 } as const;
    expect(speakersText([])).toBe("Keine Figuren");
    expect(speakersText([f({ kind: "none" }), f(local, 0, 0)])).toBe("Keine Figur spricht");
    expect(speakersText([f(local), f({ kind: "none" })])).toBe("1 Figur spricht");
    expect(speakersText([f(local), f({ kind: "trainer", id: 2 }), f({ kind: "common", script: 2001 }), f({ kind: "unreadable", reason: "x" })])).toBe("3 Figuren sprechen");
    expect(figureSummary(f(local, 2, 3))).toBe("2 Nachrichten · 3 Seiten");
    expect(figureSummary(f(local, 1, 1))).toBe("1 Nachricht · 1 Seite");
    expect(figureSummary(f(local, 0, 0))).toBe("Sagt nichts");
    expect(figureSummary(f({ kind: "trainer", id: 2 }))).toBe("Kampftexte des Trainers: im Bereich Pokémon");
    expect(figureSummary(f({ kind: "common", script: 2001 }))).toBe("Gemeinsames Script: in Bauen bearbeiten.");
    expect(figureKey({ kind: "bg", index: 0 })).toBe("bg:0");
    expect(figureTarget({ header: 12, event_file: 40 }, { kind: "bg", index: 0 })).toEqual({ header: 12, eventFile: 40, kind: "bg", index: 0, title: "Schild 0" });
  });
});
