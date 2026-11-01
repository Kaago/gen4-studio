import { describe, expect, it } from "vitest";
import type { Dialog, DialogMessage } from "../../api";
import { dialogSummary, pageCount, sharedText } from "./dialogModel";

const message = (index: number, text: string | null, over: Partial<DialogMessage> = {}): DialogMessage => ({ archive: 554, index, text, command: "Message", line: 3, col: 13, branch: null, shared_with: 0, ...over });
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

  it("shared_text_singular_and_plural", () => {
    expect(sharedText(1)).toBe("Geteilt mit 1 Figur");
    expect(sharedText(3)).toBe("Geteilt mit 3 Figuren");
  });
});
