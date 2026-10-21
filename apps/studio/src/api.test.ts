import { describe, expect, it } from "vitest";
import { isCmdError } from "./api";

describe("isCmdError", () => {
  it("recognises structured command errors", () => {
    expect(isCmdError({ code: "stale", message: "m", asset: { kind: "Map", index: 0 } })).toBe(true);
    expect(isCmdError({ code: "error", message: "m", asset: null })).toBe(true);
    expect(isCmdError({ code: "read_only", message: "m", asset: { kind: "Event", index: 3 } })).toBe(true);
    expect(isCmdError("plain string")).toBe(false);
  });

  it("rejects objects of the wrong shape", () => {
    expect(isCmdError(null)).toBe(false);
    expect(isCmdError({ code: "other", message: "m", asset: null })).toBe(false);
    expect(isCmdError({ code: "error", asset: null })).toBe(false);
  });
});
