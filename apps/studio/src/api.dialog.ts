import { invoke } from "./platform/transport";

/**
 * The dialog of a figure (3d.5), mirroring `apps/studio/src-tauri/src/commands/dialog.rs` and `g4_kernel::dialog`. `api.ts`
 * re-exports everything. Field names are the Rust ones (snake_case). `dialogOf` reads; `splitMessage` is one undo step that also
 * writes and compiles the figure's script file, and refuses (code "error", the kernel's German sentence) a message that is not
 * shared, one in a label other scripts of the file reach, or a script another figure runs too.
 */

/**
 * Where a figure's dialog comes from: script `#slot` of the header's script file `file` (`line` 1-based), a common script (2000
 * and up, edited in Bauen), a trainer's battle (its texts are trainer texts), nothing (script 0), or a source that cannot be read
 * (not adopted, binary, a parse error: `reason`).
 */
export type DialogSource =
  | { kind: "local"; file: number; slot: number; line: number }
  | { kind: "common"; script: number }
  | { kind: "trainer"; id: number }
  | { kind: "none" }
  | { kind: "unreadable"; reason: string };

/**
 * One message a figure's script shows: message `index` of text archive `archive` (`text` null when the archive has none there),
 * the command as written, the 1-based line and column of its number in the source, the condition it is shown under (null outside:
 * the `if` it stands in, or the conditional jump that reached it, `GoToIfSet FLAG_X` / `CheckFlag FLAG_X; GoToIf EQUAL`), how many
 * other figures show the same message and why `splitMessage` would refuse it (null: it can be split; the kernel's sentence else).
 */
export interface DialogMessage {
  archive: number;
  index: number;
  text: string | null;
  command: string;
  line: number;
  col: number;
  branch: string | null;
  shared_with: number;
  split_refusal: string | null;
}

/** The dialog of a figure: its messages in reading order and the labels the walk followed (`GoTo`, `Call`). */
export interface Dialog {
  source: DialogSource;
  messages: DialogMessage[];
  followed: string[];
}

/** What `splitMessage` did: message `old_index` copied to `new_index`, the source lines rewritten, the undo step (`change`). */
export interface SplitMessageReport {
  archive: number;
  old_index: number;
  new_index: number;
  change: number | null;
  lines: number[];
}

/** The dialog of event `index` (an object or a sign) of event file `eventFile`, looked up on header `header`. */
export const dialogOf = (header: number, eventFile: number, kind: "object" | "bg", index: number) =>
  invoke<Dialog>("dialog_of", { header, eventFile, kind, index });

/** Gives object `index` of `eventFile` its own copy of message (`archive`, `message`) as one undo step „Dialog von Figur k abgespalten“. */
export const splitMessage = (header: number, eventFile: number, index: number, archive: number, message: number) =>
  invoke<SplitMessageReport>("split_message", { header, eventFile, index, archive, message });

/**
 * One figure of a place with what its dialog comes to (3d.8, `dialogs_of_header`): an object (`local_id`, `graphics_id`) or a sign
 * (both null) by its index within its class, `trainer` the trainer's id for a trainer's battle, `source` as `dialogOf` names it, the
 * text of the first message (null without one or when the archive has none there), how many messages it shows (each once), how
 * many boxes they fill (`pages`) and the messages as indices into the header's text archive in the order of their first use
 * (`message_ids`, empty unless the dialog is a local script's).
 */
export interface FigureDialog {
  kind: "object" | "bg";
  index: number;
  local_id: number | null;
  graphics_id: number | null;
  trainer: number | null;
  source: DialogSource;
  first_text: string | null;
  messages: number;
  pages: number;
  message_ids: number[];
}

/** The figures of header `header`'s event file, the objects by index, then the signs; `archive` is the header's text archive. */
export interface HeaderDialogs {
  header: number;
  event_file: number;
  archive: number;
  figures: FigureDialog[];
  ms: number;
}

/** Every figure of header `header` with its dialog in one call; an event file that does not load has none. */
export const dialogsOfHeader = (header: number) => invoke<HeaderDialogs>("dialogs_of_header", { header });
