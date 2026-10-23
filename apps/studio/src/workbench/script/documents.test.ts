import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { ScriptDocuments, lineOffset, projectFileUri, rootUri, scriptUri } from "./documents";

describe("ScriptDocuments", () => {
  it("opens_a_document_clean_with_no_editor_state", () => {
    const docs = new ScriptDocuments();
    const doc = docs.open(3, "script", "script Main #1:\n\tEnd\n");
    expect(doc).toMatchObject({ file: 3, kind: "script", text: "script Main #1:\n\tEnd\n", savedText: "script Main #1:\n\tEnd\n", state: null, cursor: 0 });
    expect(docs.get(3)).toBe(doc);
    expect(docs.dirty()).toEqual([]);
  });

  it("returns_the_open_document_unchanged_when_opened_again", () => {
    const docs = new ScriptDocuments();
    const doc = docs.open(3, "script", "a");
    docs.get(3)!.text = "edited";
    expect(docs.open(3, "script", "from disk")).toBe(doc);
    expect(doc.text).toBe("edited");
    expect(doc.savedText).toBe("a");
  });

  it("is_dirty_while_the_text_differs_from_the_saved_text", () => {
    const docs = new ScriptDocuments();
    docs.open(1, "script", "a");
    docs.open(2, "levelScript", "{}");
    docs.get(2)!.text = "{ }";
    expect(docs.dirty()).toEqual([2]);
    docs.get(2)!.text = "{}";
    expect(docs.dirty()).toEqual([]);
    docs.get(1)!.text = "b";
    docs.get(1)!.savedText = "b";
    expect(docs.dirty()).toEqual([]);
    expect(docs.get(1)?.savedText).toBe("b");
  });

  it("lists_documents_in_the_order_they_were_opened_and_closes_them", () => {
    const docs = new ScriptDocuments();
    docs.open(5, "script", "");
    docs.open(2, "levelScript", "");
    docs.open(9, "script", "");
    expect(docs.list().map((d) => d.file)).toEqual([5, 2, 9]);
    docs.close(2);
    expect(docs.get(2)).toBeUndefined();
    expect(docs.list().map((d) => d.file)).toEqual([5, 9]);
  });

  it("ignores_closing_a_file_that_is_not_open", () => {
    const docs = new ScriptDocuments();
    docs.close(7);
    expect(docs.list()).toEqual([]);
  });

  it("keeps_the_editor_state_on_the_document", () => {
    const docs = new ScriptDocuments();
    const doc = docs.open(1, "script", "a");
    doc.state = EditorState.create({ doc: "a" });
    expect(docs.get(1)?.state?.doc.toString()).toBe("a");
  });
});

describe("scriptUri", () => {
  it("builds_file_uri_with_padded_number_and_unchanged_case", () => {
    expect(scriptUri("/Users/Poltep/Roms/Proj", 42)).toBe("file:///Users/Poltep/Roms/Proj/expanded/scripts/0042.rotom");
    expect(scriptUri("/r/", 7)).toBe("file:///r/expanded/scripts/0007.rotom");
  });

  it("encodes_spaces_but_not_parentheses_in_a_rom_project_path", () => {
    expect(scriptUri("/Users/POLTEP/Projekte/lab/editor/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents", 1)).toBe(
      "file:///Users/POLTEP/Projekte/lab/editor/Roms/Pokemon%20-%20Platin-Edition%20(Germany)_DSPRE_contents/expanded/scripts/0001.rotom",
    );
  });

  it("encodes_non_ascii_and_the_characters_that_would_end_the_path", () => {
    expect(scriptUri("/Users/Jörg/a%b#c?d", 3)).toBe("file:///Users/J%C3%B6rg/a%25b%23c%3Fd/expanded/scripts/0003.rotom");
  });

  it("uses_json_for_a_level_script", () => {
    expect(scriptUri("/r", 12, "levelScript")).toBe("file:///r/expanded/scripts/0012.json");
  });
});

describe("lineOffset and rootUri", () => {
  it("finds_where_a_one_based_line_starts_and_clamps", () => {
    const text = "one\ntwo\nthree";
    expect(lineOffset(text, 1)).toBe(0);
    expect(lineOffset(text, 0)).toBe(0);
    expect(lineOffset(text, 2)).toBe(4);
    expect(lineOffset(text, 3)).toBe(8);
    expect(lineOffset(text, 9)).toBe(text.length);
    expect(lineOffset("", 5)).toBe(0);
  });

  it("encodes_the_root_like_the_file_uris_do", () => {
    expect(rootUri("/Users/me/My Project/")).toBe("file:///Users/me/My%20Project");
    expect(scriptUri("/Users/me/My Project/", 1).startsWith(rootUri("/Users/me/My Project/"))).toBe(true);
    expect(rootUri("/a#b")).toBe("file:///a%23b");
    expect(projectFileUri("/Users/me/My Project", "expanded/textArchives/0199.json")).toBe("file:///Users/me/My%20Project/expanded/textArchives/0199.json");
    expect(projectFileUri("/p", "rotom.toml")).toBe("file:///p/rotom.toml");
  });
});
