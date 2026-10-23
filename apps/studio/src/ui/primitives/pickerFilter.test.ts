import { describe, expect, it } from "vitest";
import { filterItems } from "./pickerFilter";
import type { PickerItem } from "./pickerFilter";

const items: PickerItem[] = [
  { id: 410, label: "Jubelstadt", tech: "T01R0201", area: "welt" },
  { id: 411, label: "Zweiblattdorf", tech: "T01R0101", area: "welt" },
  { id: 412, label: "Sandgemme", tech: "T01R0102", area: "welt" },
  { id: 3, label: "Das Zweite Tor", area: "welt" },
  { id: "x7", label: "Zwei Brücken", area: "welt" },
];
const ids = (list: PickerItem[]) => list.map((i) => i.id);

describe("filterItems", () => {
  it("empty_query_returns_all_items_in_input_order", () => {
    expect(ids(filterItems(items, ""))).toEqual([410, 411, 412, 3, "x7"]);
    expect(ids(filterItems(items, "   "))).toEqual([410, 411, 412, 3, "x7"]);
  });

  it("matches_a_substring_of_the_label_ignoring_case", () => {
    expect(ids(filterItems(items, "blatt"))).toEqual([411]);
    expect(ids(filterItems(items, "SANDGEM"))).toEqual([412]);
  });

  it("matches_the_tech_name", () => {
    expect(ids(filterItems(items, "T01R0102"))).toEqual([412]);
    expect(ids(filterItems(items, "t01r01"))).toEqual([411, 412]);
  });

  it("a_numeric_query_also_matches_the_id_with_leading_zeros", () => {
    expect(ids(filterItems(items, "0411"))).toEqual([411]);
    expect(ids(filterItems(items, "411"))).toEqual([411]);
    expect(ids(filterItems(items, "0003"))).toEqual([3]);
  });

  it("a_numeric_query_matches_the_id_exactly_not_as_a_substring", () => {
    expect(ids(filterItems(items, "41"))).toEqual([]);
  });

  it("a_text_query_never_matches_the_id", () => {
    expect(ids(filterItems(items, "x7"))).toEqual([]);
  });

  it("prefix_matches_come_before_substring_matches_each_in_input_order", () => {
    // "zwei" starts "Zweiblattdorf" and "Zwei Brücken" but only occurs inside "Das Zweite Tor".
    expect(ids(filterItems(items, "zwei"))).toEqual([411, "x7", 3]);
  });

  it("a_prefix_of_the_tech_name_counts_as_a_prefix_match", () => {
    const list: PickerItem[] = [
      { id: 1, label: "Hat T01 im Namen", area: "welt" },
      { id: 2, label: "Anders", tech: "T01R9", area: "welt" },
    ];
    expect(ids(filterItems(list, "t01"))).toEqual([2, 1]);
  });

  it("an_id_match_ranks_with_the_prefix_matches", () => {
    const list: PickerItem[] = [
      { id: 1, label: "Stufe 411 Nord", area: "welt" },
      { id: 411, label: "Ziel", area: "welt" },
    ];
    expect(ids(filterItems(list, "411"))).toEqual([411, 1]);
  });

  it("returns_nothing_when_nothing_matches", () => {
    expect(filterItems(items, "qqq")).toEqual([]);
  });
});
