import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { ScriptDocuments, scriptUri } from "./documents";

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
    docs.setText(3, "edited");
    expect(docs.open(3, "script", "from disk")).toBe(doc);
    expect(doc.text).toBe("edited");
    expect(doc.savedText).toBe("a");
  });

  it("is_dirty_while_the_text_differs_from_the_saved_text", () => {
    const docs = new ScriptDocuments();
    docs.open(1, "script", "a");
    docs.open(2, "levelScript", "{}");
    docs.setText(2, "{ }");
    expect(docs.dirty()).toEqual([2]);
    docs.setText(2, "{}");
    expect(docs.dirty()).toEqual([]);
    docs.setText(1, "b");
    docs.markSaved(1);
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

  it("ignores_text_and_save_calls_for_a_file_that_is_not_open", () => {
    const docs = new ScriptDocuments();
    docs.setText(7, "x");
    docs.markSaved(7);
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
