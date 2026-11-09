import type { HeaderSummary } from "../../api";
import { UNNAMED_KEY, buildPlaces } from "../places";
import { id4, termLabel } from "../terms";

/** The archive and message a link led to (`message` null: the first one). */
export interface TextTarget {
  index: number;
  message: number | null;
}

/** One entry of the archive list: the archive, the headers that use it and the place it is named after (null: none). */
export interface ArchiveRow {
  index: number;
  users: number[];
  place: string | null;
}

/** How many hits the text search asks for. */
export const SEARCH_LIMIT = 50;

/** How many characters a line of the text box holds, as the preview counts them (`lineWidth`): a longer line is marked as too long. */
export const LINE_LIMIT = 38;

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

/** How many characters the widest line of a message holds, over all its pages (`lineWidth`); 0 for an empty text. */
export function longestLine(text: string): number {
  let widest = 0;
  for (const page of pages(text)) for (const line of page.lines) widest = Math.max(widest, lineWidth(line));
  return widest;
}

/** The text with real line breaks (Enter, a paste) written as the `\n` the archive stores: a message holds no raw break. */
export const toTextForm = (text: string) => text.replace(/\r\n|\r|\n/g, "\\n");

/** "Textarchiv 0199". */
export const archiveLabel = (index: number) => `${termLabel("textArchive")} ${id4(index)}`;

/** A message number with three digits ("005"), as the table and the heads write it. */
export const id3 = (index: number) => String(index).padStart(3, "0");

/** "Nachricht 005". */
export const messageLabel = (index: number) => `${termLabel("message")} ${id3(index)}`;

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

/**
 * The name of each used archive: the place (`buildPlaces`) of the first header that uses it. An archive whose first header has no
 * location name has none.
 */
export function archivePlaces(headers: readonly HeaderSummary[], users: ReadonlyMap<number, number[]>): Map<number, string> {
  const placeOf = new Map<number, string>();
  for (const place of buildPlaces([...headers], [])) {
    if (place.key === UNNAMED_KEY) continue;
    for (const entry of place.headers) placeOf.set(entry.header.id, place.name);
  }
  const names = new Map<number, string>();
  for (const [archive, ids] of users) {
    const name = ids.length > 0 ? placeOf.get(ids[0]!) : undefined;
    if (name !== undefined) names.set(archive, name);
  }
  return names;
}

/** One row per archive of the project (`count` is the member count; undefined while it is not known). */
export function archiveRows(count: number | undefined, users: ReadonlyMap<number, number[]>, places: ReadonlyMap<number, string> = new Map()): ArchiveRow[] {
  return Array.from({ length: count ?? 0 }, (_, index) => ({ index, users: users.get(index) ?? [], place: places.get(index) ?? null }));
}

/** Whether the search text is a number (an archive or header number). */
export const isNumericQuery = (query: string) => /^\d+$/.test(query.trim());

/**
 * The archives the list's filter keeps: a number matches the archive number and the number of every header that uses it, any
 * text matches the place an archive is named after (ignoring case); an empty query keeps every archive.
 */
export function filterArchives(rows: ArchiveRow[], query: string): ArchiveRow[] {
  const q = query.trim();
  if (q === "") return rows;
  const numeric = isNumericQuery(q);
  const has = (n: number) => id4(n).includes(q);
  const lower = q.toLowerCase();
  return rows.filter((r) => (numeric && (has(r.index) || r.users.some(has))) || (r.place !== null && r.place.toLowerCase().includes(lower)));
}
