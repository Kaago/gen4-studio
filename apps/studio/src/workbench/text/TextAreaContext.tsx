import { createContext, useContext } from "react";
import type { FigureDialog, HeaderSummary, ProjectInfo, TextArchiveDto, TextHit } from "../../api";
import type { ScriptFiles } from "../../data/useDataArea";
import type { Depth } from "../../ui/frame/shellState";
import type { NavTarget } from "../navigation";
import type { Place } from "../places";
import type { ProblemsStore } from "../problems/problems";
import type { ArchiveRow } from "./textModel";
import type { FoundHeaderDialogs } from "./useHeaderDialogs";

/** Where the hand's search looks: the open archive (filtered here) or all archives (the backend's search). */
export type TextScope = "archive" | "all";

/** What the text area has chosen: the parts of the area (list, view, hand, status) read and change it here. */
export interface TextAreaState {
  /** The open archive; null: none. */
  archive: number | null;
  /** The selected message of the open archive (clamped to its messages once they are read); null: none. */
  message: number | null;
  /** The page of the selected message the view shows (0-based; the view clamps it to the pages there are). */
  page: number;
  /** The hand's search. */
  query: string;
  scope: TextScope;
  /** Opens `archive` with message `message` (default: the first) on its first page. */
  select(archive: number, message?: number): void;
  setPage(page: number): void;
  setQuery(query: string): void;
  setScope(scope: TextScope): void;
}

/**
 * The text typed into the message field before it is committed, for the view to follow at once. A plain store for
 * `useSyncExternalStore`: the field writes to it per keystroke, only the view reads it, so typing renders nothing else.
 */
export class TextDraft {
  private shown: { key: string; text: string } | null = null;
  private readonly listeners = new Set<() => void>();

  /** The typed text of message `key` ("archive:message"); null while nothing was typed into it. */
  textOf(key: string): string | null {
    return this.shown?.key === key ? this.shown.text : null;
  }

  get = () => this.shown;

  set(key: string, text: string): void {
    if (this.shown?.key === key && this.shown.text === text) return;
    this.shown = { key, text };
    for (const listener of [...this.listeners]) listener();
  }

  /** Forgets the typed text (another project was opened). */
  clear(): void {
    if (this.shown === null) return;
    this.shown = null;
    for (const listener of [...this.listeners]) listener();
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };
}

/** The key of a message in `TextDraft`. */
export const draftKey = (archive: number, message: number) => `${archive}:${message}`;

/** The state of the area plus what its parts show and do: the archives, the open one, the search hits and the edits. */
export interface TextAreaModel extends TextAreaState {
  project: ProjectInfo | null;
  headers: readonly HeaderSummary[];
  /** The member count of the archives; undefined while it is not known (no archive can be created then). */
  archiveCount: number | undefined;
  /** One row per archive of the project, named after its place. */
  rows: ArchiveRow[];
  /** The headers that use each archive. */
  users: ReadonlyMap<number, number[]>;
  /** The open archive as read (unsaved edits included); null while it loads, when it cannot be read or none is open. */
  loaded: TextArchiveDto | null;
  /** Why the open archive cannot be read. */
  error: string | null;
  loading: boolean;
  /** The longest line of each message of the open archive (`longestLine`), in message order. */
  widths: readonly number[];
  /** The scope the search really has: all archives while none is open. */
  effectiveScope: TextScope;
  /** The hits of the search over all archives; null while there is no such search or it has not answered yet. */
  hits: TextHit[] | null;
  searchError: string | null;
  problems: ProblemsStore;
  draft: TextDraft;
  /** Whether the message field has the focus (the status line names its keys then). */
  editorFocused: boolean;
  setEditorFocused(focused: boolean): void;
  /** Writes `text` into message `message` of `archive`, for project `owner` (dropped when another project is open by then). */
  commit(owner: ProjectInfo | null, archive: number, message: number, text: string): Promise<boolean>;
  /** Appends a message to the open archive and selects it. */
  append(): void;
  /** Removes the last message of the open archive. */
  removeLast(): void;
  /** Appends an archive to the project and opens it. */
  createArchive(): void;
  goTo(target: NavTarget): void;
  /** The words of the depth: Bauen names the users of an archive places, Tüfteln headers. */
  depth: Depth;
  /** The area shows Erzählen (its depth is below Bauen): dialogs by place and figure instead of archives and numbers. */
  narrating: boolean;
  /** The places of the world (`buildPlaces`), for the dialogs by place. */
  places: Place[];
  /** The header whose figures the view shows (Erzählen); null: none chosen. */
  dialogHeader: number | null;
  /** The place of `dialogHeader`. */
  dialogPlace: Place | null;
  /** The figures of `dialogHeader` as read (`useHeaderDialogs`). */
  dialogs: FoundHeaderDialogs;
  /** The chosen figure of `dialogHeader`; null: none. */
  dialogFigure: FigureDialog | null;
  /** Changes when the figures or their dialogs may have moved (texts, scripts, events): they are read again. */
  dialogRevision: number;
  /** Shows the figures of `header` (no figure chosen). */
  selectDialogHeader(header: number): void;
  /** Chooses a figure of `dialogHeader` (null: none); the hand shows its messages. */
  selectFigure(figure: FigureDialog | null): void;
  /** Opens the dialog editor of a figure of `dialogHeader`. */
  openDialogEditor(figure: FigureDialog): void;
  /** The title of a figure's card: „Objekt 3“, „Schild 0“, „Trainer Wanderer Jens“. */
  figureTitle(figure: FigureDialog): string;
  /** Asks for the trainer rows (a trainer figure is named by class and name). */
  needTrainers(): void;
  /** A text or a split of a figure's dialog was written in the hand: the app counts the edit, the texts read again. */
  dialogChanged(): void;
  onError(e: unknown): void;
  /** The script panel's documents, for a split in the hand. */
  scriptFiles: ScriptFiles | undefined;
}

export const TextAreaContext = createContext<TextAreaModel | null>(null);

/** The model of the text area around the caller (`TextArea`). */
export function useTextArea(): TextAreaModel {
  const model = useContext(TextAreaContext);
  if (model === null) throw new Error("useTextArea outside TextArea");
  return model;
}
