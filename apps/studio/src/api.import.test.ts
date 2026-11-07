import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { exportMapPart, importMapParts, inspectMapImport } from "./api";

describe("map import commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("calls each command with the argument names of the backend", async () => {
    await inspectMapImport(104, { files: ["/p/route.nsbmd", "/p/route.per"] });
    expect(invoke).toHaveBeenLastCalledWith("inspect_map_import", { map: 104, source: { files: ["/p/route.nsbmd", "/p/route.per"] } });
    await inspectMapImport(104, { map: 5 });
    expect(invoke).toHaveBeenLastCalledWith("inspect_map_import", { map: 104, source: { map: 5 } });
    await importMapParts(104, { map: 5 }, ["model", "permissions"]);
    expect(invoke).toHaveBeenLastCalledWith("import_map_parts", {
      map: 104,
      source: { map: 5 },
      parts: ["model", "permissions"],
      keepBuildings: false,
    });
    await importMapParts(104, { files: ["/p/hg.bin"] }, ["whole"], true);
    expect(invoke).toHaveBeenLastCalledWith("import_map_parts", {
      map: 104,
      source: { files: ["/p/hg.bin"] },
      parts: ["whole"],
      keepBuildings: true,
    });
    await exportMapPart(6, "bdhc", "/p/0006.bdhc");
    expect(invoke).toHaveBeenLastCalledWith("export_map_part", { map: 6, part: "bdhc", path: "/p/0006.bdhc" });
  });

  it("passes the conversion options only when they are given", async () => {
    await inspectMapImport(187, { files: ["/p/a.obj"] }, { unit: 16, colors: 4 });
    expect(invoke).toHaveBeenLastCalledWith("inspect_map_import", { map: 187, source: { files: ["/p/a.obj"] }, options: { unit: 16, colors: 4 } });
    await inspectMapImport(187, { files: ["/p/a.obj"] });
    expect(invoke).toHaveBeenLastCalledWith("inspect_map_import", { map: 187, source: { files: ["/p/a.obj"] } });
    expect(invoke.mock.lastCall?.[1]).not.toHaveProperty("options");
    await importMapParts(187, { files: ["/p/a.obj", "/p/b.png"] }, ["model", "textures"], false, { colors: 16 });
    expect(invoke).toHaveBeenLastCalledWith("import_map_parts", {
      map: 187,
      source: { files: ["/p/a.obj", "/p/b.png"] },
      parts: ["model", "textures"],
      keepBuildings: false,
      options: { colors: 16 },
    });
    await importMapParts(187, { files: ["/p/a.obj"] }, ["model"]);
    expect(invoke.mock.lastCall?.[1]).not.toHaveProperty("options");
  });

  it("passes the reports and changed assets through", async () => {
    const report = [{ part: "model", file: "/p/a.nsbmd", size: 1024, state: { kind: "warning", code: "model.unaligned", args: ["1022"], message: "model length 1022 is not a multiple of 4" }, notes: [] }];
    invoke.mockResolvedValueOnce(report);
    await expect(inspectMapImport(1, { files: ["/p/a.nsbmd"] })).resolves.toEqual(report);
    invoke.mockResolvedValueOnce([{ kind: "Map", index: 1 }]);
    await expect(importMapParts(1, { files: ["/p/a.nsbmd"] }, ["model"])).resolves.toEqual([{ kind: "Map", index: 1 }]);
  });
});
