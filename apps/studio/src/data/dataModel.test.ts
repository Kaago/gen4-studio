import { describe, expect, it } from "vitest";
import { lockReason, matchesQuery } from "./dataModel";
import { species } from "./dataTesting";

describe("matchesQuery", () => {
  const find = (query: string, indices: number[]) => indices.filter((i) => matchesQuery(species(i, `Form ${i % 7}`), `Form ${i % 7}`, query));

  it("finds_a_number_and_the_four_digit_numbers_that_start_with_it_like_the_backend", () => {
    const all = [5, 50, 117, 150, 170, 500, 507];
    // "050" starts 0500 to 0509 and is 50; "50" is 50 and starts no four-digit number here (not 0150 or 0500, which only contain it).
    expect(find("050", all)).toEqual([50, 500, 507]);
    expect(find("50", all)).toEqual([50]);
    // "17": the member 17 (not listed) and nothing that merely contains it.
    expect(find("17", all)).toEqual([]);
    expect(find("011", all)).toEqual([117]);
  });

  it("finds_a_name_by_any_part_in_any_case", () => {
    expect(matchesQuery(species(397, "Staravia"), "Staravia", "RAVI")).toBe(true);
    expect(matchesQuery(species(397, "Staravia"), "Staravia", "  ")).toBe(true);
    expect(matchesQuery(species(397, "Staravia"), "Staravia", "Bisa")).toBe(false);
  });
});

describe("lockReason", () => {
  it("puts_the_kernels_reason_into_german_by_the_part_it_names", () => {
    expect(lockReason("item 0000: 36 bytes, expected 34")).toBe("Daten 0000: 36 Bytes statt 34");
    expect(lockReason("move 0470: 18 bytes, expected 16")).toBe("Attacke 0470: 18 Bytes statt 16");
    expect(lockReason("personal data 0397: 36 bytes, expected 44")).toBe("Pokémon-Daten 0397: 36 Bytes statt 44");
    expect(lockReason("something else")).toBe("something else");
  });
});
