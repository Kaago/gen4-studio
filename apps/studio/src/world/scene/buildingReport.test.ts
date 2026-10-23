import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { buildingsOf } from "./buildingReport";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

const building = (model_id: number) => ({ model_id, x: 0, y: 0, z: 0, sx: 1, sy: 1, sz: 1, rotation: ["0", "0", "0"] });
const loaded = (index: number, models: number[]) => [index, { map: { index, permissions: [], buildings: models.map(building) } } as unknown as Loaded] as const;

describe("buildingsOf", () => {
  it("lists_the_maps_with_buildings_in_matrix_order_once_each_and_skips_empty_and_missing_maps", () => {
    const matrix = { index: 0, name: "m", width: 3, height: 2, maps: [4, null, 2, 4, 9, 7] } as MatrixDto;
    const h = { matrix, loaded: new Map([loaded(4, [10, 11]), loaded(2, []), loaded(7, [3])]) };
    expect(buildingsOf(h as unknown as SceneHost)).toEqual([
      { map: 4, models: [10, 11] },
      { map: 7, models: [3] },
    ]);
  });

  it("is_empty_without_a_matrix", () => {
    expect(buildingsOf({ matrix: null, loaded: new Map() } as unknown as SceneHost)).toEqual([]);
  });
});
