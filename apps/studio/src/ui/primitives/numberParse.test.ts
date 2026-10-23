import { describe, expect, it } from "vitest";
import { checkNumber, formatNumber, parseNumber } from "./numberParse";

describe("parseNumber", () => {
  it("parses_hex_with_and_without_prefix_in_any_case", () => {
    expect(parseNumber("0x1F", "hex")).toBe(31);
    expect(parseNumber("1f", "hex")).toBe(31);
    expect(parseNumber(" 0X1f ", "hex")).toBe(31);
  });

  it("parses_decimal_integers_and_a_leading_minus", () => {
    expect(parseNumber("42", "dec")).toBe(42);
    expect(parseNumber("-7", "dec")).toBe(-7);
    expect(Object.is(parseNumber("-0", "dec"), 0)).toBe(true);
  });

  it("refuses_anything_that_is_not_a_whole_number_of_that_base", () => {
    expect(parseNumber("0xZZ", "hex")).toBeNull();
    expect(parseNumber("", "dec")).toBeNull();
    expect(parseNumber("   ", "hex")).toBeNull();
    expect(parseNumber("12.5", "dec")).toBeNull();
    expect(parseNumber("1e3", "dec")).toBeNull();
    expect(parseNumber("0x1F", "dec")).toBeNull();
    expect(parseNumber("0x", "hex")).toBeNull();
    expect(parseNumber("12abc", "dec")).toBeNull();
  });
});

describe("formatNumber", () => {
  it("writes_hex_with_prefix_and_capitals", () => {
    expect(formatNumber(255, "hex")).toBe("0xFF");
    expect(formatNumber(0, "hex")).toBe("0x0");
    expect(formatNumber(-16, "hex")).toBe("-0x10");
  });
  it("writes_decimal_plain", () => {
    expect(formatNumber(65535, "dec")).toBe("65535");
  });
});

describe("checkNumber", () => {
  it("accepts_a_value_inside_the_bounds", () => {
    expect(checkNumber("65535", "dec", 0, 65535)).toEqual({ ok: true, value: 65535 });
    expect(checkNumber("0", "dec", 0, 65535)).toEqual({ ok: true, value: 0 });
  });

  it("names_the_maximum_for_a_value_above_it_and_never_clamps", () => {
    expect(checkNumber("70000", "dec", 0, 65535)).toEqual({ ok: false, error: "höchstens 65535" });
  });

  it("names_the_minimum_for_a_value_below_it", () => {
    expect(checkNumber("-1", "dec", 0, 65535)).toEqual({ ok: false, error: "mindestens 0" });
    expect(checkNumber("3", "dec", 5, undefined)).toEqual({ ok: false, error: "mindestens 5" });
  });

  it("writes_the_bound_in_the_base_of_the_field", () => {
    expect(checkNumber("0x10000", "hex", 0, 65535)).toEqual({ ok: false, error: "höchstens 0xFFFF" });
  });

  it("explains_unreadable_and_empty_input_with_a_way_out", () => {
    const hex = checkNumber("0xZZ", "hex", 0, 255);
    expect(hex.ok).toBe(false);
    expect(!hex.ok && hex.error).toContain("0x1F");
    const empty = checkNumber("", "dec", 0, 255);
    expect(empty.ok).toBe(false);
    expect(!empty.ok && empty.error).toContain("Zahl");
  });

  it("has_no_upper_bound_without_a_maximum", () => {
    expect(checkNumber("99999999", "dec", 0, undefined)).toEqual({ ok: true, value: 99999999 });
  });
});
