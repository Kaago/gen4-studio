import { describe, expect, it } from "vitest";
import type { AssetId } from "../../api";
import { isDirty } from "./dirtyAssets";

describe("isDirty", () => {
  const dirty: AssetId[] = [
    { kind: "Event", index: 8 },
    { kind: "Matrix", index: 3 },
  ];

  it("is_true_for_the_asset_of_that_kind_and_index", () => {
    expect(isDirty(dirty, "Event", 8)).toBe(true);
    expect(isDirty(dirty, "Matrix", 3)).toBe(true);
  });

  it("is_false_for_another_index_or_another_kind_with_the_same_index", () => {
    expect(isDirty(dirty, "Event", 3)).toBe(false);
    expect(isDirty(dirty, "Map", 8)).toBe(false);
    expect(isDirty([], "Event", 8)).toBe(false);
  });
});
