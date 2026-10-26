import { describe, expect, it } from "vitest";
import { coverRect } from "./snapshot";

describe("coverRect", () => {
  it("cuts a wide picture left and right and a tall one top and bottom, keeping the centre", () => {
    expect(coverRect(1600, 900, 512, 384)).toEqual({ sx: 200, sy: 0, sw: 1200, sh: 900 });
    expect(coverRect(900, 1600, 512, 384)).toEqual({ sx: 0, sy: 462.5, sw: 900, sh: 675 });
  });

  it("takes a picture of the frame's shape whole", () => {
    expect(coverRect(1024, 768, 512, 384)).toEqual({ sx: 0, sy: 0, sw: 1024, sh: 768 });
  });
});
