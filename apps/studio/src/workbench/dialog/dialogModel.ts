import type { Dialog, DialogMessage, EventKind } from "../../api";
import { pages } from "../text/textModel";

/** The figure whose dialog the inspector shows and the dialog editor edits: an object or a sign of `eventFile`, looked up on `header`. */
export interface DialogTarget {
  header: number;
  eventFile: number;
  kind: "object" | "bg";
  index: number;
  /** The figure's name in the head: „Objekt 3“ / „Schild 0“. */
  title: string;
}

/** „Objekt 3“, „Schild 0“: what the dialog editor's title calls the figure. */
export const dialogTitle = (kind: Extract<EventKind, "object" | "bg">, index: number) => `${kind === "bg" ? "Schild" : "Objekt"} ${index}`;

/** „Geteilt mit 2 Figuren“: the badge of a message other figures show too. */
export const sharedText = (n: number) => `Geteilt mit ${n} ${n === 1 ? "Figur" : "Figuren"}`;

/** A message of a dialog once: its first use in reading order and the other uses of the same (archive, index). */
export interface DistinctMessage {
  message: DialogMessage;
  others: DialogMessage[];
}

/**
 * The dialog's messages, each (archive, index) once in the order of its first use: `dialog_of` lists every use (a message shown on two
 * lines comes twice), the editor and the summary count a message once.
 */
export function distinctMessages(messages: readonly DialogMessage[]): DistinctMessage[] {
  const byKey = new Map<string, DistinctMessage>();
  for (const m of messages) {
    const key = `${m.archive}:${m.index}`;
    const seen = byKey.get(key);
    if (seen === undefined) byKey.set(key, { message: m, others: [] });
    else seen.others.push(m);
  }
  return [...byKey.values()];
}

/** How many boxes the messages fill in the game (`pages`); a message the archive does not have fills none. */
export function pageCount(messages: readonly DialogMessage[]): number {
  return messages.reduce((sum, m) => sum + (m.text === null ? 0 : pages(m.text).length), 0);
}

/**
 * The line under the preview: „2 Nachrichten · 3 Seiten“ (each message once, however often the script shows it) for a script of the figure's own, „Sagt nichts“ for none (or a script without a
 * message), and for a source the dialog editor does not edit, where it is edited.
 */
export function dialogSummary(d: Dialog): string {
  const source = d.source;
  switch (source.kind) {
    case "common":
      return `Gemeinsames Script ${source.script}: in Bauen bearbeiten.`;
    case "trainer":
      return "Kampftexte des Trainers: im Bereich Pokémon.";
    case "unreadable":
      return `Script nicht lesbar: ${source.reason}`;
    case "none":
      return "Sagt nichts";
    case "local": {
      const distinct = distinctMessages(d.messages).map((x) => x.message);
      const n = distinct.length;
      if (n === 0) return "Sagt nichts";
      const m = pageCount(distinct);
      return `${n} ${n === 1 ? "Nachricht" : "Nachrichten"} · ${m} ${m === 1 ? "Seite" : "Seiten"}`;
    }
  }
}
