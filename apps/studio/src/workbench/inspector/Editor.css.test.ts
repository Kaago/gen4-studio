import { describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../../ui/cssSource";

const css = readCss("./Editor.module.css", import.meta.url);

describe("a field with buttons next to it", () => {
  it("keeps a usable width and lets the buttons wrap instead of squeezing the control", () => {
    const grow = ruleBody(css, ".grow");
    // A flex basis above zero is what makes a wrapping row break before the control shrinks.
    expect(grow).toMatch(/flex:\s*1\s+1\s+(?!0(?:px|rem)?\b)\S+/);
    expect(grow).toMatch(/min-width:\s*(?!0\b)\S+/);
    expect(ruleBody(css, ".inline")).toMatch(/flex-wrap:\s*wrap/);
  });
});
