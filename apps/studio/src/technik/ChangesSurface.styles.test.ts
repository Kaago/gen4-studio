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

  it("keeps_the_git_setup_button_at_its_own_width", () => {
    // `.surface > *` stretches every child to the reading width; the offer is a flex row, so its button sizes to its content.
    expect(ruleBody(css, ".offer")).toMatch(/display:\s*flex/);
  });

  it("puts_the_detail_beside_the_list_from_760px_of_the_split", () => {
    // The split is the container its columns ask; below 760 px one column (the detail under the list), from 760 px two.
    expect(ruleBody(css, ".surface > .split")).toMatch(/container-type:\s*inline-size/);
    expect(ruleBody(css, ".columns")).toMatch(/grid-template-columns:\s*minmax\(0,\s*1fr\)\s*;/);
    expect(css).toMatch(/@container \(min-width: 760px\) \{\s*\.columns \{\s*grid-template-columns:\s*minmax\(0,\s*1fr\)\s+minmax\(0,\s*1fr\)/);
  });
});
