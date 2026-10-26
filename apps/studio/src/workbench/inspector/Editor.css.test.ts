import { describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../../ui/cssSource";

const css = readCss("./Editor.module.css", import.meta.url);

describe("the inspector's frame", () => {
  it("is_the_container_its_rows_ask_for_their_width", () => {
    // `InspectorRow` puts the label above its control while this container is narrow (the Werkbank's inspector).
    expect(ruleBody(css, ".panel")).toMatch(/container:\s*inspector\s*\/\s*inline-size/);
  });
});

describe("a field with buttons next to it", () => {
  it("keeps a usable width and lets the buttons wrap instead of squeezing the control", () => {
    const grow = ruleBody(css, ".grow");
    // A flex basis above zero is what makes a wrapping row break before the control shrinks.
    expect(grow).toMatch(/flex:\s*1\s+1\s+(?!0(?:px|rem)?\b)\S+/);
    expect(grow).toMatch(/min-width:\s*(?!0\b)\S+/);
    expect(ruleBody(css, ".inline")).toMatch(/flex-wrap:\s*wrap/);
  });
});
