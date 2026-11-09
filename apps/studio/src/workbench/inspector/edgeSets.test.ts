import { describe, expect, it } from "vitest";
import { EDGE_PLACES } from "../../api.world";
import type { EdgeSetJson } from "../../api.world";
import { PLACE_GRID, PLACE_LABEL, completeness, newSet, placeTiles, setTiles, withVariant, withoutVariant } from "./edgeSets";

const weg = (): EdgeSetJson => ({ name: "Weg", outside: "0000", places: { center: ["0006"], n: ["0007r1"] } });

describe("edgeSets", () => {
  it("grid_holds_all_fifteen_places_once", () => {
    const flat = PLACE_GRID.flat().filter((p) => p !== null);
    expect(flat).toHaveLength(15);
    expect(new Set(flat)).toEqual(new Set(EDGE_PLACES));
    // The 3 × 3 block, the 2 × 2 block and the row of the diagonals.
    expect(PLACE_GRID[0]).toEqual(["outer_nw", "n", "outer_ne"]);
    expect(PLACE_GRID[1]).toEqual(["w", "center", "e"]);
    expect(PLACE_GRID[2]).toEqual(["outer_sw", "s", "outer_se"]);
    expect(PLACE_LABEL.outer_nw).toBe("Ecke außen NW");
    expect(PLACE_LABEL.e).toBe("Kante O");
    expect(PLACE_LABEL.inner_se).toBe("Ecke innen SO");
    expect(PLACE_LABEL.diag_nwse).toBe("Diagonale NW–SO");
    expect(PLACE_LABEL.center).toBe("Mitte");
  });

  it("variants_append_once_and_remove", () => {
    const set = weg();
    const more = withVariant(set, "n", "0008");
    expect(more.places.n).toEqual(["0007r1", "0008"]);
    expect(set.places.n).toEqual(["0007r1"]);
    // The same token (any case) is not appended twice.
    expect(withVariant(more, "n", "0008").places.n).toEqual(["0007r1", "0008"]);
    expect(withVariant(set, "n", "0007R1").places.n).toEqual(["0007r1"]);
    expect(withVariant(set, "outer_nw", "0009").places.outer_nw).toEqual(["0009"]);
    // Removing the last variant leaves the place out.
    const less = withoutVariant(set, "n", "0007r1");
    expect(less.places).toEqual({ center: ["0006"] });
    expect("n" in less.places).toBe(false);
    expect(withoutVariant(more, "n", "0007r1").places.n).toEqual(["0008"]);
  });

  it("completeness_counts_thirteen_and_two", () => {
    expect(completeness(weg())).toEqual({ places: 2, diagonals: 0 });
    const full: EdgeSetJson = { name: "Weg", outside: "0000", places: Object.fromEntries(EDGE_PLACES.map((p, i) => [p, [(i + 6).toString(16).padStart(4, "0")]])) };
    expect(completeness(full)).toEqual({ places: 13, diagonals: 2 });
    expect(completeness({ ...full, places: { ...full.places, diag_nesw: [] } })).toEqual({ places: 13, diagonals: 1 });
  });

  it("a_new_set_is_empty_with_its_outside", () => {
    expect(newSet("Satz 1", "0002r1")).toEqual({ name: "Satz 1", outside: "0002r1", places: {} });
  });

  it("place_tiles_are_the_distinct_tiles_of_the_places", () => {
    const set: EdgeSetJson = { name: "Weg", outside: "0000", places: { center: ["0006", "000a"], n: ["0006r1"], s: ["0010f"] } };
    expect(placeTiles(set)).toEqual([6, 10, 16]);
  });

  it("set_tiles_are_the_places_and_the_outside", () => {
    const set: EdgeSetJson = { name: "Weg", outside: "0012r1", places: { center: ["0006", "000a"], n: ["0006r1"], s: ["0010f"] } };
    expect(setTiles(set)).toEqual([6, 10, 16, 18]);
    // The outside counted once when a place holds it too; an empty set is its outside alone.
    expect(setTiles({ ...set, outside: "0006" })).toEqual([6, 10, 16]);
    expect(setTiles({ name: "Fels", outside: "0001", places: {} })).toEqual([1]);
  });
});
