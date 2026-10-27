import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import { projectPreviewGet, projectPreviewSet } from "../api";
import type { PreviewDto } from "../api";

describe("api.preview", () => {
  beforeEach(() => invoke.mockReset());

  it("projectPreviewGet_sends_path_null_without_a_path", async () => {
    const none: PreviewDto = { source: null, png: null, standardMap: null };
    invoke.mockResolvedValue(none);
    await expect(projectPreviewGet()).resolves.toBe(none);
    await projectPreviewGet("/p/Platin");
    expect(invoke.mock.calls).toEqual([
      ["project_preview_get", { path: null }],
      ["project_preview_get", { path: "/p/Platin" }],
    ]);
  });

  it("projectPreviewSet_sends_every_key_null_unless_given", async () => {
    invoke.mockResolvedValue({ source: "standard", png: "iVBO", standardMap: { header: 411, map: 3, areaDataId: 1 } });
    await projectPreviewSet({ png: "iVBO" });
    await projectPreviewSet({ path: "/p/Platin", slot: "standard", file: "/bilder/titel.jpg" });
    await projectPreviewSet({});
    expect(invoke.mock.calls).toEqual([
      ["project_preview_set", { path: null, slot: null, png: "iVBO", file: null }],
      ["project_preview_set", { path: "/p/Platin", slot: "standard", png: null, file: "/bilder/titel.jpg" }],
      ["project_preview_set", { path: null, slot: null, png: null, file: null }],
    ]);
  });
});
