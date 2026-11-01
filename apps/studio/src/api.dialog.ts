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
 * the command as written, the 1-based line and column of its number in the source, the condition of the `if` it stands in (null
 * outside) and how many other figures show the same message.
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
