import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../../api";
import { archiveLabel, archiveRows, filterArchives, hitLabel, isNumericQuery, segments, toTextForm, usersByArchive, usersLabel, visibleLine } from "./textModel";

const header = (id: number, text_archive_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
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
      { kind: "scroll", text: "\\r" },
      { kind: "clear", text: "\\f" },
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

  it("lists the headers that use an archive, a few of them in full", () => {
    expect(usersLabel([])).toBe("");
    expect(usersLabel([415])).toBe("Header 0415");
    expect(usersLabel([415, 416])).toBe("Header 0415, 0416");
    expect(usersLabel([1, 2, 3, 4, 5])).toBe("Header 0001, 0002, 0003 +2");
  });

  it("labels a hit with archive, message and excerpt", () => {
    expect(hitLabel({ archive: 199, index: 12, excerpt: "…Hallo…" })).toBe("0199 · 12 · …Hallo…");
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
      { index: 0, users: [] },
      { index: 1, users: [] },
      { index: 2, users: [7] },
      { index: 3, users: [] },
    ]);
    expect(archiveRows(undefined, new Map())).toEqual([]);
  });
});

describe("filterArchives", () => {
  const rows = archiveRows(500, usersByArchive([header(415, 199), header(416, 199), header(9, 300)]));

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

  it("matches no archive for a query that is not a number (the text search answers that)", () => {
    expect(isNumericQuery("0199")).toBe(true);
    expect(isNumericQuery(" 12 ")).toBe(true);
    expect(isNumericQuery("Ruhe")).toBe(false);
    expect(isNumericQuery("")).toBe(false);
    expect(filterArchives(rows, "Ruhe")).toEqual([]);
  });
});
