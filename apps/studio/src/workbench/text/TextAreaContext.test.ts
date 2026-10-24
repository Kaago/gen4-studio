import { describe, expect, it, vi } from "vitest";
import { TextDraft, draftKey } from "./TextAreaContext";

describe("TextDraft", () => {
  it("keeps the typed text of one message and forgets it when cleared", () => {
    const draft = new TextDraft();
    const heard = vi.fn();
    draft.subscribe(heard);
    draft.set(draftKey(433, 1), "Neu");
    expect(draft.textOf("433:1")).toBe("Neu");
    expect(draft.textOf("433:2")).toBeNull();
    draft.set("433:1", "Neu");
    expect(heard).toHaveBeenCalledTimes(1);
    draft.clear();
    expect(draft.get()).toBeNull();
    expect(draft.textOf("433:1")).toBeNull();
    expect(heard).toHaveBeenCalledTimes(2);
    // Clearing what is clear tells nobody.
    draft.clear();
    expect(heard).toHaveBeenCalledTimes(2);
  });
});
