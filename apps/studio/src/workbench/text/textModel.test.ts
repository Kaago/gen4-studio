import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../../api";
import { LINE_LIMIT, archiveLabel, archivePlaces, archiveRows, filterArchives, isNumericQuery, lineWidth, longestLine, messageLabel, pages, segments, toTextForm, usersByArchive, visibleLine } from "./textModel";

const header = (id: number, text_archive_id: number, location_name = ""): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id,
  script_file_id: 0,
  level_script_id: 0,
});

describe("visibleLine", () => {
  // Messages hold chatot's text form: a line break is a backslash and an "n", not a control character.
  it("makes the line break, scroll and clear codes of a message visible", () => {
    expect(visibleLine("a\\nb")).toBe("a⏎b");
    expect(visibleLine("a\\rb")).toBe("a▼b");
    expect(visibleLine("a\\fb")).toBe("a▽b");
    expect(visibleLine("Hallo\\n\\rWelt\\f")).toBe("Hallo⏎▼Welt▽");
  });

  it("leaves other text, empty text and other codes alone", () => {
    expect(visibleLine("")).toBe("");
    expect(visibleLine("Zweiblattdorf")).toBe("Zweiblattdorf");
    expect(visibleLine("{COLOR, 0, 5}Rot[PK]")).toBe("{COLOR, 0, 5}Rot[PK]");
    // An escaped backslash is not the start of a line break.
    expect(visibleLine("a\\\\nb")).toBe("a\\\\nb");
  });
});

describe("segments", () => {
  it("splits a message into text, the three break codes and other codes", () => {
    expect(segments("Hi {PLAYER}\\n[PK]x\\r\\f")).toEqual([
      { kind: "text", text: "Hi " },
      { kind: "code", text: "{PLAYER}" },
      { kind: "newline", text: "\\n" },
      { kind: "code", text: "[PK]" },
      { kind: "text", text: "x" },
      { kind: "clear", text: "\\r" },
      { kind: "scroll", text: "\\f" },
    ]);
    expect(segments("\\x0123!")).toEqual([
      { kind: "code", text: "\\x0123" },
      { kind: "text", text: "!" },
    ]);
    expect(segments("")).toEqual([]);
    expect(segments("a\\\\nb")).toEqual([{ kind: "text", text: "a\\\\nb" }]);
  });

  it("keeps the text of every segment, in order", () => {
    const text = "{A, 1}x\\ny[B]\\\\z\\f";
    expect(segments(text).map((s) => s.text).join("")).toBe(text);
  });
});

describe("toTextForm", () => {
  it("turns real line breaks (typed or pasted) into the escape the archive stores", () => {
    expect(toTextForm("a\nb")).toBe("a\\nb");
    expect(toTextForm("a\r\nb\rc")).toBe("a\\nb\\nc");
    expect(toTextForm("kein Umbruch")).toBe("kein Umbruch");
  });
});

describe("labels", () => {
  it("writes archive numbers with four digits and the glossary's word", () => {
    expect(archiveLabel(199)).toBe("Textarchiv 0199");
  });

  it("writes message numbers with three digits", () => {
    expect(messageLabel(5)).toBe("Nachricht 005");
    expect(messageLabel(1234)).toBe("Nachricht 1234");
  });
});

describe("archive rows", () => {
  it("groups the headers by the archive they use, in header order", () => {
    const users = usersByArchive([header(0, 433), header(1, 5), header(2, 433)]);
    expect(users.get(433)).toEqual([0, 2]);
    expect(users.get(5)).toEqual([1]);
    expect(users.get(6)).toBeUndefined();
  });

  it("makes one row per archive of the project", () => {
    const rows = archiveRows(4, usersByArchive([header(7, 2)]));
    expect(rows).toEqual([
      { index: 0, users: [], place: null },
      { index: 1, users: [], place: null },
      { index: 2, users: [7], place: null },
      { index: 3, users: [], place: null },
    ]);
    expect(archiveRows(undefined, new Map())).toEqual([]);
  });

  it("names an archive after the place of the first header that uses it; a nameless place gives no name", () => {
    const headers = [header(3, 412, "Zweiblattdorf"), header(4, 412, "Sandgemme"), header(5, 7, " "), header(6, 8, "Jubelstadt")];
    const users = usersByArchive(headers);
    const places = archivePlaces(headers, users);
    expect(places.get(412)).toBe("Zweiblattdorf");
    expect(places.has(7)).toBe(false);
    expect(archiveRows(413, users, places)[412]).toEqual({ index: 412, users: [3, 4], place: "Zweiblattdorf" });
  });
});

describe("filterArchives", () => {
  const headers = [header(415, 199), header(416, 199), header(9, 300), header(3, 412, "Zweiblattdorf")];
  const users = usersByArchive(headers);
  const rows = archiveRows(500, users, archivePlaces(headers, users));

  it("keeps every archive for an empty query", () => {
    expect(filterArchives(rows, "")).toBe(rows);
    expect(filterArchives(rows, "   ")).toBe(rows);
  });

  it("matches the archive number, written with or without leading zeros", () => {
    expect(filterArchives(rows, "0199").map((r) => r.index)).toEqual([199]);
    expect(filterArchives(rows, "199").map((r) => r.index)).toEqual([199]);
  });

  it("matches the number of a header that uses the archive", () => {
    expect(filterArchives(rows, "0416").map((r) => r.index)).toContain(199);
    expect(filterArchives(rows, "0009").map((r) => r.index)).toContain(300);
  });

  it("matches the place name of an archive, in any case, and nothing else for a word", () => {
    expect(isNumericQuery("0199")).toBe(true);
    expect(isNumericQuery(" 12 ")).toBe(true);
    expect(isNumericQuery("Ruhe")).toBe(false);
    expect(isNumericQuery("")).toBe(false);
    expect(filterArchives(rows, "Zweiblatt").map((r) => r.index)).toEqual([412]);
    expect(filterArchives(rows, "zweiBLATTdorf").map((r) => r.index)).toEqual([412]);
    expect(filterArchives(rows, "Ruhe")).toEqual([]);
  });
});

const shown = (text: string) => pages(text).map((p) => p.lines.map((l) => l.map((s) => s.text).join("")));

describe("pages", () => {
  it("pages_split_on_clear_and_two_lines_per_page", () => {
    expect(shown(String.raw`A\nB\rC`)).toEqual([["A", "B"], ["C"]]);
    expect(pages(String.raw`A\nB\rC`)[0].prompt).toBe(true);
  });
  it("scroll_keeps_the_last_line", () => {
    expect(shown(String.raw`A\nB\fC`)).toEqual([["A", "B"], ["B", "C"]]);
    expect(pages(String.raw`A\nB\fC`)[0].prompt).toBe(true);
  });
  it("a_third_newline_overflows_into_a_new_page", () => {
    const p = pages(String.raw`A\nB\nC`);
    expect(p).toHaveLength(2);
    expect(p[0].overflow).toBe(false);
    expect(p[1].overflow).toBe(true);
  });
  it("codes_stay_in_the_line_and_count_zero", () => {
    const p = pages("Hallo {PLAYER}!");
    expect(p[0].lines[0].some((s) => s.kind === "code")).toBe(true);
    expect(lineWidth(p[0].lines[0])).toBe(7);
  });
  it("trailing_break_makes_no_empty_page", () => {
    expect(pages(String.raw`A\r`)).toHaveLength(1);
    expect(pages(String.raw`A\nB\f`)).toHaveLength(1);
    expect(pages(String.raw`A\r`)[0].prompt).toBe(true);
  });
  it("empty_text_is_one_page_with_one_empty_line", () => {
    expect(shown("")).toEqual([[""]]);
    expect(pages("")[0]).toMatchObject({ prompt: false, overflow: false });
  });
});

describe("lineWidth", () => {
  it("counts_glyph_codes_one_and_control_codes_zero", () => {
    expect(lineWidth(pages("[PK]MON\\x0123{COLOR, 0, 5}")[0].lines[0])).toBe(1 + 3 + 1);
  });
  it("counts_an_escaped_backslash_one", () => {
    expect(lineWidth(pages(String.raw`a\\b`)[0].lines[0])).toBe(3);
  });
});

describe("longestLine", () => {
  it("is_the_width_of_the_widest_line_of_any_page", () => {
    expect(longestLine(String.raw`Hallo\nWelt, wie geht es?\rKurz`)).toBe(18);
    expect(longestLine("")).toBe(0);
    expect(longestLine("x".repeat(45)) > LINE_LIMIT).toBe(true);
    expect(LINE_LIMIT).toBe(38);
  });
});
