// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getAreaData } from "../api";
import type { AreaDataDto } from "../api";
import { useBuildingModelKind } from "./useBuildingModelKind";
import type { BuildingModelKind } from "./useBuildingModelKind";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  getAreaData: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useBuildingModelKind", () => {
  let root: Root;
  let shown: BuildingModelKind | null;

  function Probe({ areaDataId }: { areaDataId: number | null }) {
    shown = useBuildingModelKind(areaDataId, 0);
    return null;
  }
  const show = (areaDataId: number | null) => act(async () => root.render(<Probe areaDataId={areaDataId} />));

  beforeEach(() => {
    vi.mocked(getAreaData).mockReset();
    root = createRoot(document.createElement("div"));
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("takes_the_interior_archive_for_an_hgss_interior_area", async () => {
    vi.mocked(getAreaData).mockResolvedValue({ family: "Hgss", area_type: 0 } as AreaDataDto);
    await show(3);
    expect(shown).toBe("interior_building_model");
  });

  it("falls_back_to_the_exterior_archive_like_the_backend_without_area_data_or_when_it_cannot_be_read", async () => {
    // No area data: the exterior archive at once, nothing read.
    await show(null);
    expect(shown).toBe("building_model");
    expect(getAreaData).not.toHaveBeenCalled();
    // Unreadable area data: the backend takes the exterior archive too (`building_inputs`).
    vi.mocked(getAreaData).mockRejectedValue({ code: "error", message: "kaputt", asset: null });
    await show(5);
    expect(shown).toBe("building_model");
  });

  it("is_null_while_the_area_data_is_read", async () => {
    vi.mocked(getAreaData).mockReturnValue(new Promise(() => {}));
    await show(7);
    expect(shown).toBeNull();
  });
});
