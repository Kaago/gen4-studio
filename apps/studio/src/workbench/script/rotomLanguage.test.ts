import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { rotomLanguage, tokenize } from "./rotomLanguage";

describe("tokenize", () => {
  it("styles_a_script_header_with_keyword_name_and_label", () => {
    expect(tokenize("script Main #1:")).toEqual([
      { text: "script", style: "keyword" },
      { text: "Main", style: "variableName" },
      { text: "#1", style: "number" },
      { text: ":", style: "" },
    ]);
  });

  it("styles_a_command_with_a_string_and_a_trailing_comment", () => {
    expect(tokenize('Message "Hi" // c')).toEqual([
      { text: "Message", style: "command" },
      { text: '"Hi"', style: "string" },
      { text: "// c", style: "comment" },
    ]);
  });

  it("names_the_definition_after_function_and_action_too", () => {
    expect(tokenize("function Helper")).toEqual([
      { text: "function", style: "keyword" },
      { text: "Helper", style: "variableName" },
    ]);
    expect(tokenize("action Walk")[1]).toEqual({ text: "Walk", style: "variableName" });
  });

  it("styles_every_control_keyword", () => {
    for (const word of ["script", "function", "action", "if", "else", "while", "match"]) {
      expect(tokenize(word)).toEqual([{ text: word, style: "keyword" }]);
    }
  });

  it("styles_decimal_and_hex_numbers_and_lowercase_names_as_variables", () => {
    expect(tokenize("SetVar flag 0x1F 12")).toEqual([
      { text: "SetVar", style: "command" },
      { text: "flag", style: "variableName" },
      { text: "0x1F", style: "number" },
      { text: "12", style: "number" },
    ]);
  });

  it("marks_include_and_define_as_meta_but_not_a_label", () => {
    expect(tokenize("#include")).toEqual([{ text: "#include", style: "meta" }]);
    expect(tokenize("#define X 1")).toEqual([
      { text: "#define", style: "meta" },
      { text: "X", style: "command" },
      { text: "1", style: "number" },
    ]);
    expect(tokenize("#12")).toEqual([{ text: "#12", style: "number" }]);
  });

  it("keeps_a_comment_marker_inside_a_string_and_handles_escapes", () => {
    expect(tokenize('Message "a // b \\" c"')).toEqual([
      { text: "Message", style: "command" },
      { text: '"a // b \\" c"', style: "string" },
    ]);
  });

  it("styles_an_unterminated_string_up_to_the_end_of_the_line", () => {
    expect(tokenize('Message "open')).toEqual([
      { text: "Message", style: "command" },
      { text: '"open', style: "string" },
    ]);
  });

  it("resets_the_definition_state_on_each_line", () => {
    expect(tokenize("script")).toEqual([{ text: "script", style: "keyword" }]);
    expect(tokenize("Main")).toEqual([{ text: "Main", style: "command" }]);
  });
});

describe("rotomLanguage", () => {
  it("is_named_rotom_for_the_language_server", () => {
    expect(rotomLanguage.name).toBe("rotom");
  });

  it("parses_a_document_and_offers_line_comment_tokens_for_toggling", () => {
    const state = EditorState.create({ doc: "script Main #1:\n\tEnd // done\n", extensions: [rotomLanguage] });
    expect(state.languageDataAt("commentTokens", 0)).toEqual([{ line: "//" }]);
  });
});
