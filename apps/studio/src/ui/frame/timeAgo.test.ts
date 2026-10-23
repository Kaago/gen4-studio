import { describe, expect, it } from "vitest";
import { ago } from "./timeAgo";

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

describe("ago", () => {
  it("says_just_now_below_a_minute_and_for_the_future", () => {
    expect(ago(1000, 1000)).toBe("gerade eben");
    expect(ago(0, 59_999)).toBe("gerade eben");
    expect(ago(5000, 1000)).toBe("gerade eben");
  });

  it("counts_minutes_hours_and_days_with_the_right_number_word", () => {
    expect(ago(0, MIN)).toBe("vor 1 Minute");
    expect(ago(0, 4 * MIN + 30_000)).toBe("vor 4 Minuten");
    expect(ago(0, HOUR)).toBe("vor 1 Stunde");
    expect(ago(0, 2 * HOUR)).toBe("vor 2 Stunden");
    expect(ago(0, DAY)).toBe("vor 1 Tag");
    expect(ago(0, 3 * DAY)).toBe("vor 3 Tagen");
  });
});
