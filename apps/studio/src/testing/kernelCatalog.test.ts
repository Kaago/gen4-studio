import { describe, expect, it } from "vitest";
import { kernelSentence } from "./kernelCatalog";

describe("kernelSentence", () => {
  it("reads_a_sentence_of_each_catalog_by_section_and_key", () => {
    expect(kernelSentence("en", "git", "first_commit")).toBe("Initial state");
    expect(kernelSentence("de", "textproject", "and_more")).toBe(" und %{n} weitere");
    expect(kernelSentence("en", "git", "not_installed")).toMatch(/^Git is not installed\./);
    expect(kernelSentence("de", "git", "not_installed")).toMatch(/^Git ist nicht installiert\./);
  });

  it("says_undefined_for_a_missing_section_or_key", () => {
    expect(kernelSentence("en", "no_such_section", "first_commit")).toBeUndefined();
    expect(kernelSentence("en", "git", "no_such_key")).toBeUndefined();
    // A key of another section is not found under this one.
    expect(kernelSentence("en", "git", "revert_pending")).toBeUndefined();
    expect(kernelSentence("de", "textproject", "not_installed")).toBeUndefined();
  });

  it("takes_the_key_whole_not_as_a_pattern_or_a_prefix", () => {
    expect(kernelSentence("en", "git", "first")).toBeUndefined();
    expect(kernelSentence("en", "git", "first.commit")).toBeUndefined();
    expect(kernelSentence("en", "g.t", "first_commit")).toBeUndefined();
  });
});
