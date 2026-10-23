import type { HeaderSummary, TextHit } from "../../api";
import { TERMS, id4 } from "../terms";

/** The archive and message a link led to (`message` null: the first one). */
export interface TextTarget {
  index: number;
  message: number | null;
}

/** One entry of the archive list: the archive and the headers that use it. */
export interface ArchiveRow {
  index: number;
  users: number[];
}

/** How many header numbers a row writes out before it says "+N". */
const MAX_USERS_SHOWN = 3;

/** How many hits the text search asks for. */
export const SEARCH_LIMIT = 50;

export type SegmentKind = "text" | "newline" | "scroll" | "clear" | "code";

export interface Segment {
  kind: SegmentKind;
  text: string;
}

/** An unknown code (`\xNNNN`), a backslash pair (`\n`, `\r`, `\f`, an escaped backslash) or a control code in braces or brackets (`{COLOR, 0, 5}`, `[PK]`). */
const TOKEN = /\\x[0-9a-fA-F]{4}|\\([\s\S])|\{[^{}]*\}|\[[^[\]]*\]/g;
const BREAKS: Record<string, SegmentKind> = { n: "newline", r: "clear", f: "scroll" };

/**
 * A message in chatot's text form split into plain text, the line break (`\n`), clear (`\r`) and
 * scroll (`\f`) codes and other control codes. The segments joined give the text back.
 */
export function segments(text: string): Segment[] {
  const found: Segment[] = [];
  let plain = "";
  const push = (kind: SegmentKind, piece: string) => {
    if (kind === "text") {
      plain += piece;
      return;
    }
    if (plain !== "") found.push({ kind: "text", text: plain });
    plain = "";
    found.push({ kind, text: piece });
  };
  let last = 0;
  for (const m of text.matchAll(TOKEN)) {
    push("text", text.slice(last, m.index));
    last = m.index + m[0].length;
    // `\\` and a backslash before any other letter stay text; only the three break codes are codes.
    push(m[1] !== undefined ? (BREAKS[m[1]] ?? "text") : "code", m[0]);
  }
  push("text", text.slice(last));
  if (plain !== "") found.push({ kind: "text", text: plain });
  return found;
}

const SYMBOLS: Partial<Record<SegmentKind, string>> = { newline: "⏎", clear: "▼", scroll: "▽" };

/** A message on one line: the line break, clear and scroll codes get a visible stand-in (⏎ ▼ ▽). */
export function visibleLine(text: string): string {
  return segments(text)
    .map((s) => SYMBOLS[s.kind] ?? s.text)
    .join("");
}

/** One box of the dialog as the game shows it: two lines, whether it waits for a key (`prompt`) and whether it only exists because a third line did not fit (`overflow`). */
export interface Page {
  lines: Segment[][];
  prompt: boolean;
  overflow: boolean;
}

/**
 * A message cut into the boxes the game shows. `\n` moves from line 1 to line 2; a `\n` in line 2
 * does not fit and starts a new page (`overflow`). `\r` (clear, ▼) ends the page with a prompt and the
 * next one starts empty; `\f` (scroll, ▽) ends it with a prompt and the next one starts with the old
 * line 2 as its line 1. A `\r` or `\f` at the very end adds no empty page; an empty text is one page
 * with one empty line.
 */
export function pages(text: string): Page[] {
  const result: Page[] = [];
  let page: Page = { lines: [[]], prompt: false, overflow: false };
  // True while the current page only exists because a clear/scroll code ended the one before and nothing followed yet.
  let fresh = false;
  const line = () => page.lines[page.lines.length - 1];
  const next = (lines: Segment[][], overflow: boolean) => {
    result.push(page);
    page = { lines, prompt: false, overflow };
  };
  for (const segment of segments(text)) {
    if (segment.kind === "newline") {
      if (page.lines.length === 1) page.lines.push([]);
      else next([[]], true);
      fresh = false;
    } else if (segment.kind === "clear" || segment.kind === "scroll") {
      page.prompt = true;
      next(segment.kind === "scroll" ? [page.lines[1] ?? [], []] : [[]], false);
      fresh = true;
    } else {
      line().push(segment);
      fresh = false;
    }
  }
  if (!fresh) result.push(page);
  return result;
}

const GLYPH_CODE = /^(\[[^[\]]*\]|\\x[0-9a-fA-F]{4})$/;

/** How many characters a line holds: text counts its characters (an escaped backslash one), `[PK]` and `\xNNNN` one each, control codes in braces none. */
export function lineWidth(line: Segment[]): number {
  let width = 0;
  for (const s of line) {
    if (s.kind === "text") width += Array.from(s.text.replace(/\\\\/g, "\\")).length;
    else if (s.kind === "code" && GLYPH_CODE.test(s.text)) width += 1;
  }
  return width;
}

/** The text with real line breaks (Enter, a paste) written as the `\n` the archive stores: a message holds no raw break. */
export const toTextForm = (text: string) => text.replace(/\r\n|\r|\n/g, "\\n");

/** "Textarchiv 0199". */
export const archiveLabel = (index: number) => `${TERMS.textArchive.label} ${id4(index)}`;

/** "Header 0415, 0416" (empty for an archive no header uses); long lists end in "+N". */
export function usersLabel(users: readonly number[]): string {
  if (users.length === 0) return "";
  const shown = users.slice(0, MAX_USERS_SHOWN).map(id4).join(", ");
  const more = users.length - MAX_USERS_SHOWN;
  return `${TERMS.header.label} ${shown}${more > 0 ? ` +${more}` : ""}`;
}

/** A search hit as the list shows it: archive, message number, excerpt. */
export const hitLabel = (hit: TextHit) => `${id4(hit.archive)} · ${hit.index} · ${hit.excerpt}`;

/** The headers (ids, in header order) that use each text archive, from the header table the app already holds. */
export function usersByArchive(headers: readonly HeaderSummary[]): Map<number, number[]> {
  const users = new Map<number, number[]>();
  for (const h of headers) {
    const list = users.get(h.text_archive_id);
    if (list) list.push(h.id);
    else users.set(h.text_archive_id, [h.id]);
  }
  return users;
}

/** One row per archive of the project (`count` is the member count; undefined while it is not known). */
export function archiveRows(count: number | undefined, users: ReadonlyMap<number, number[]>): ArchiveRow[] {
  return Array.from({ length: count ?? 0 }, (_, index) => ({ index, users: users.get(index) ?? [] }));
}

/** Whether the search text is a number (an archive or header number); anything else is a text search. */
export const isNumericQuery = (query: string) => /^\d+$/.test(query.trim());

/**
 * The archives a search box filters the list to: a number matches the archive number and the
 * number of every header that uses it, an empty query keeps every archive, any other text matches
 * none (the full-text search lists its hits instead).
 */
export function filterArchives(rows: ArchiveRow[], query: string): ArchiveRow[] {
  const q = query.trim();
  if (q === "") return rows;
  if (!isNumericQuery(q)) return [];
  const has = (n: number) => id4(n).includes(q);
  return rows.filter((r) => has(r.index) || r.users.some(has));
}
