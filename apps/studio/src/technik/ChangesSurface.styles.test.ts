import { describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../ui/cssSource";

const css = readCss("./ChangesSurface.module.css", import.meta.url);

describe("the Änderungen surface", () => {
  it("scrolls instead_of_clipping_the_card_of_groups", () => {
    // jsdom has no layout: the card clips with `overflow: hidden` (rounded corners), which sets a flex item's
    // automatic minimum height to 0. Without `flex-shrink: 0` it shrinks to the space left and cuts its groups off.
    expect(ruleBody(css, ".surface")).toMatch(/overflow:\s*auto/);
    const groups = ruleBody(css, ".groups");
    expect(groups).toMatch(/overflow:\s*hidden/);
    expect(groups).toMatch(/flex-shrink:\s*0|flex:\s*none|flex:\s*0\s+0/);
  });
});
