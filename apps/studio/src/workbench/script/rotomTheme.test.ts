// @vitest-environment jsdom
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { describe, expect, it } from "vitest";
import { syntax } from "../../ui/tokens/tokens";
import { ROTOM_HIGHLIGHT, ROTOM_THEME } from "./rotomTheme";

const colourOf = (mode: "hell" | "dunkel", tag: unknown) => {
  const hit = ROTOM_HIGHLIGHT[mode].find((s) => (Array.isArray(s.tag) ? s.tag.includes(tag as never) : s.tag === tag));
  return hit?.color;
};

describe("rotomTheme", () => {
  it("colours_a_comment_with_the_syntax_token_of_the_mode", () => {
    expect(colourOf("dunkel", tags.comment)).toBe(syntax.dunkel.kommentar);
    expect(colourOf("hell", tags.comment)).toBe(syntax.hell.kommentar);
  });

  it("colours_keywords_strings_and_numbers_from_the_syntax_tokens", () => {
    for (const mode of ["hell", "dunkel"] as const) {
      expect(colourOf(mode, tags.keyword)).toBe(syntax[mode].schluesselwort);
      expect(colourOf(mode, tags.string)).toBe(syntax[mode].zeichenkette);
      expect(colourOf(mode, tags.number)).toBe(syntax[mode].zahl);
      expect(colourOf(mode, tags.meta)).toBe(syntax[mode].flag);
      expect(colourOf(mode, tags.invalid)).toBe(syntax[mode].fehler);
    }
  });

  it("underlines_information_and_hints_in_the_info_colour_not_the_success_colour", () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const view = new EditorView({ state: EditorState.create({ extensions: ROTOM_THEME.hell }), parent });
    try {
      const css = [...document.querySelectorAll("style")].map((s) => s.textContent ?? "").join("\n") +
        (document.adoptedStyleSheets ?? []).flatMap((sheet) => [...sheet.cssRules].map((r) => r.cssText)).join("\n");
      const rulesOf = (selector: string) => css.split("}").filter((rule) => rule.includes(selector));
      for (const selector of [".cm-lintRange-info", ".cm-lintRange-hint", ".cm-diagnostic-info", ".cm-diagnostic-hint"]) {
        const rules = rulesOf(selector);
        expect(rules.length, selector).toBeGreaterThan(0);
        expect(rules.some((r) => r.includes("--g4-status-info")), selector).toBe(true);
        expect(rules.some((r) => r.includes("--g4-status-erfolg")), selector).toBe(false);
      }
    } finally {
      view.destroy();
      parent.remove();
    }
  });

  it("marks_only_the_dark_theme_as_dark", () => {
    expect(EditorState.create({ extensions: ROTOM_THEME.dunkel }).facet(EditorView.darkTheme)).toBe(true);
    expect(EditorState.create({ extensions: ROTOM_THEME.hell }).facet(EditorView.darkTheme)).toBe(false);
  });
});
