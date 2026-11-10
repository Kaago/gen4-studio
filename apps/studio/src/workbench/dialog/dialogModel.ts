import type { Dialog, DialogMessage, EventKind, FigureDialog, HeaderDialogs } from "../../api";
import { t } from "../../i18n";
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
export const dialogTitle = (kind: Extract<EventKind, "object" | "bg">, index: number) =>
  kind === "bg" ? t("workbench:dialogModel.sign", { index }) : t("workbench:dialogModel.object", { index });

/** „Geteilt mit 2 Figuren“: the badge of a message other figures show too. */
export const sharedText = (n: number) => t("workbench:dialogModel.shared", { count: n });

/** „2 Nachrichten · 3 Seiten“: how many messages a dialog has and how many boxes they fill. */
const countsText = (messages: number, pageCount: number) =>
  t("workbench:dialogModel.counts", { messages: t("workbench:dialogModel.messages", { count: messages }), pages: t("workbench:dialogModel.pages", { count: pageCount }) });

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
      return t("workbench:dialogModel.commonScriptNumber", { script: source.script });
    case "trainer":
      return t("workbench:dialogModel.trainerTexts");
    case "unreadable":
      return t("workbench:dialogModel.unreadable", { reason: source.reason });
    case "none":
      return t("workbench:dialogModel.saysNothing");
    case "local": {
      const distinct = distinctMessages(d.messages).map((x) => x.message);
      const n = distinct.length;
      if (n === 0) return t("workbench:dialogModel.saysNothing");
      return countsText(n, pageCount(distinct));
    }
  }
}

/** A figure of a place by class and index: „object:3“, „bg:0“ (the cards' `data-dialog-figure`). */
export const figureKey = (f: Pick<FigureDialog, "kind" | "index">) => `${f.kind}:${f.index}`;

/** The dialog editor's target for figure `f` of the place `dialogs` lists. */
export const figureTarget = (dialogs: Pick<HeaderDialogs, "header" | "event_file">, f: Pick<FigureDialog, "kind" | "index">): DialogTarget => ({
  header: dialogs.header,
  eventFile: dialogs.event_file,
  kind: f.kind,
  index: f.index,
  title: dialogTitle(f.kind, f.index),
});

/** Whether a figure says something: its own script shows a message, or a common script or a trainer's battle speaks for it. */
export function speaks(f: FigureDialog): boolean {
  switch (f.source.kind) {
    case "local":
      return f.messages > 0;
    case "common":
    case "trainer":
      return true;
    case "none":
    case "unreadable":
      return false;
  }
}

/** The line of a place in the list of dialogs: „2 Figuren sprechen“, „1 Figur spricht“, „Keine Figur spricht“, „Keine Figuren“. */
export function speakersText(figures: readonly FigureDialog[]): string {
  if (figures.length === 0) return t("workbench:dialogModel.noFigures");
  const n = figures.filter(speaks).length;
  if (n === 0) return t("workbench:dialogModel.noneSpeaks");
  return t("workbench:dialogModel.speakers", { count: n });
}

/**
 * The line of a figure's card (Erzählen, hence without numbers), like `dialogSummary` from what `dialogsOfHeader` counted: „2 Nachrichten ·
 * 3 Seiten“, „Sagt nichts“, and for a source the dialog editor does not edit where it is edited.
 */
export function figureSummary(f: FigureDialog): string {
  const source = f.source;
  switch (source.kind) {
    case "common":
      return t("workbench:dialogModel.commonScript");
    case "trainer":
      return t("workbench:dialogModel.trainerTextsShort");
    case "unreadable":
      return t("workbench:dialogModel.unreadable", { reason: source.reason });
    case "none":
      return t("workbench:dialogModel.saysNothing");
    case "local": {
      const n = f.messages;
      if (n === 0) return t("workbench:dialogModel.saysNothing");
      return countsText(n, f.pages);
    }
  }
}
