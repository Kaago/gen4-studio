import { describe, expect, it } from "vitest";
import { NBSP, WJ } from "./words";

describe("words", () => {
  it("binds_a_count_to_what_it_counts", () => {
    // UI-test fix round 1, M7: the no-break space (U+00A0) between a count and its unit; the plural keys of the catalogs carry it.
    expect(NBSP).toBe(" ");
  });

  it("joins_a_hyphenated_word_with_word_joiners", () => {
    // Acceptance 4.3b A5: U+2060 (the UI font has no U+2011), so „Basis\u2060-\u2060ROM“ never breaks inside the word.
    expect(WJ).toBe("\u2060");
  });
});
