import { describe, expect, it } from "vitest";
import { popoverPlace } from "./TermsPopover";

describe("popoverPlace", () => {
  it("keeps_the_glossary_inside_the_window_when_the_button_is_near_its_left_edge", () => {
    // Gate 3a (F): in a 300 px panel at the left, the box hung off to the left and lost its first letters.
    expect(popoverPlace({ left: 200, right: 270, bottom: 40 }, 1200)).toMatchObject({ position: "fixed", left: 8, top: 44, width: 280 });
  });

  it("ends_at_the_buttons_right_edge_when_there_is_room", () => {
    expect(popoverPlace({ left: 900, right: 970, bottom: 40 }, 1200)).toMatchObject({ left: 690, width: 280 });
  });

  it("stays_inside_a_window_narrower_than_the_box", () => {
    expect(popoverPlace({ left: 10, right: 80, bottom: 40 }, 250)).toMatchObject({ left: 8, width: 234 });
    expect(popoverPlace({ left: 1150, right: 1300, bottom: 40 }, 1200)).toMatchObject({ left: 912 });
  });
});
