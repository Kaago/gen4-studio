import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import {
  getHeader,
  headerFamily,
  memberCounts,
  headerFields,
  isCmdError,
  setHeader,
  type MapHeader,
  type MapHeaderHGSS,
  type MapHeaderPt,
} from "./api";

describe("isCmdError", () => {
  it("recognises structured command errors", () => {
    expect(isCmdError({ code: "stale", message: "m", asset: { kind: "Map", index: 0 } })).toBe(true);
    expect(isCmdError({ code: "error", message: "m", asset: null })).toBe(true);
    expect(isCmdError({ code: "read_only", message: "m", asset: { kind: "Event", index: 3 } })).toBe(true);
    expect(isCmdError("plain string")).toBe(false);
  });

  it("rejects objects of the wrong shape", () => {
    expect(isCmdError(null)).toBe(false);
    expect(isCmdError({ code: "other", message: "m", asset: null })).toBe(false);
    expect(isCmdError({ code: "error", asset: null })).toBe(false);
  });
});

const pt: MapHeaderPt = {
  area_data_id: 1,
  unknown1: 0,
  matrix_id: 2,
  script_file_id: 3,
  level_script_id: 4,
  text_archive_id: 5,
  music_day_id: 6,
  music_night_id: 7,
  wild_pokemon: 8,
  event_file_id: 9,
  location_name: 10,
  area_icon: 11,
  weather_id: 12,
  camera_angle_id: 13,
  location_specifier: 14,
  battle_background: 15,
  flags: 1,
};

const hgss: MapHeaderHGSS = {
  wild_pokemon: 1,
  area_data_id: 2,
  unknown0: 0,
  worldmap_x: 3,
  worldmap_y: 4,
  matrix_id: 5,
  script_file_id: 6,
  level_script_id: 7,
  text_archive_id: 8,
  music_day_id: 9,
  music_night_id: 10,
  event_file_id: 11,
  location_name: 12,
  area_icon: 13,
  unknown1: 0,
  kanto_flag: true,
  weather_id: 14,
  location_type: 15,
  camera_angle_id: 16,
  follow_mode: 1,
  battle_background: 17,
  flags: 0,
};

describe("map header helpers", () => {
  it("reads the family tag and the fields of every variant", () => {
    const { area_icon: _icon, ...dp } = pt;
    const headers: MapHeader[] = [{ Pt: pt }, { DP: { ...dp, location_name: 300 } }, { HGSS: hgss }];
    expect(headers.map(headerFamily)).toEqual(["Pt", "DP", "HGSS"]);
    expect(headerFields(headers[0])).toBe(pt);
    expect(headerFields(headers[1]).location_name).toBe(300);
    expect(headerFields(headers[2])).toBe(hgss);
  });
});

describe("header commands", () => {
  beforeEach(() => invoke.mockReset());

  it("reads and writes a header by id with the tagged form", async () => {
    invoke.mockResolvedValue({ Pt: pt });
    expect(await getHeader(7)).toEqual({ Pt: pt });
    expect(invoke).toHaveBeenCalledWith("get_header", { id: 7 });
    invoke.mockResolvedValue(undefined);
    await setHeader(7, { Pt: pt });
    expect(invoke).toHaveBeenCalledWith("set_header", { id: 7, header: { Pt: pt } });
  });

  it("names the undo entry after the edited field and asks for member counts", async () => {
    invoke.mockResolvedValue(undefined);
    await setHeader(7, { Pt: pt }, "Matrix");
    expect(invoke).toHaveBeenCalledWith("set_header", { id: 7, header: { Pt: pt }, label: "Matrix" });
    invoke.mockResolvedValue({ Matrix: 4, HeaderTable: 1 });
    expect(await memberCounts()).toEqual({ Matrix: 4, HeaderTable: 1 });
    expect(invoke).toHaveBeenLastCalledWith("member_counts");
  });
});
