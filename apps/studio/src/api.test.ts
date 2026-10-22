import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import {
  addBuilding,
  addScript,
  adoptScripts,
  createTextArchive,
  deleteBuilding,
  editTextArchive,
  getBuildingGlb,
  getTextArchive,
  getHeader,
  headerFamily,
  memberCounts,
  headerFields,
  isCmdError,
  lspSend,
  lspStart,
  lspStop,
  readScript,
  resolveScript,
  saveLevelScript,
  saveScript,
  scriptsStatus,
  searchText,
  setHeader,
  updateBuilding,
  validateText,
  type MapHeader,
  type MapHeaderHGSS,
  type MapHeaderPt,
} from "./api";

describe("isCmdError", () => {
  it("recognises structured command errors", () => {
    expect(isCmdError({ code: "stale", message: "m", asset: { kind: "Map", index: 0 } })).toBe(true);
    expect(isCmdError({ code: "error", message: "m", asset: null })).toBe(true);
    expect(isCmdError({ code: "read_only", message: "m", asset: { kind: "Event", index: 3 } })).toBe(true);
    expect(isCmdError({ code: "partial", message: "map 0003 was created; header edit failed: boom", asset: null })).toBe(true);
    expect(isCmdError("plain string")).toBe(false);
  });

  it("accepts the compile code with its problems", () => {
    const problem = { severity: "error", file: "expanded/scripts/0000.rotom", line: 2, col: 5, message: "m", source: "compile" };
    const error = { code: "compile", message: "1 Fehler in Scripts oder Texten", asset: null, problems: [problem] };
    expect(isCmdError(error)).toBe(true);
    expect(isCmdError({ ...error, problems: [] })).toBe(true);
    expect(isCmdError({ ...error, problems: "none" })).toBe(false);
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

describe("building commands", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("passes ids and the patch through under the commands' argument names", async () => {
    invoke.mockResolvedValue(undefined);
    await updateBuilding(3, 1, { x: -13.5, sy: 2 });
    expect(invoke).toHaveBeenLastCalledWith("update_building", { map: 3, index: 1, patch: { x: -13.5, sy: 2 } });
    await deleteBuilding(3, 1);
    expect(invoke).toHaveBeenLastCalledWith("delete_building", { map: 3, index: 1 });
    invoke.mockResolvedValue(4);
    expect(await addBuilding(3)).toBe(4);
    expect(invoke).toHaveBeenLastCalledWith("add_building", { map: 3 });
  });

  it("shares a running model request but asks again once it settled", async () => {
    const releases: ((v: ArrayBuffer) => void)[] = [];
    invoke.mockImplementation(() => new Promise<ArrayBuffer>((r) => releases.push(r)));
    const a = getBuildingGlb(5, 2);
    const b = getBuildingGlb(5, 2);
    const other = getBuildingGlb(5, 3);
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(invoke).toHaveBeenCalledWith("get_building_glb", { modelId: 5, areaDataId: 2 });
    const glb = new ArrayBuffer(8);
    releases.forEach((release) => release(glb));
    expect(await a).toBe(glb);
    expect(await b).toBe(glb);
    await other;
    invoke.mockResolvedValue(new ArrayBuffer(1));
    await getBuildingGlb(5, 2);
    expect(invoke).toHaveBeenCalledTimes(3);
  });

  it("does not remember a failed request", async () => {
    invoke.mockRejectedValueOnce({ code: "error", message: "building model 9 does not exist", asset: null });
    await expect(getBuildingGlb(9, 0)).rejects.toMatchObject({ message: "building model 9 does not exist" });
    invoke.mockResolvedValue(new ArrayBuffer(1));
    await expect(getBuildingGlb(9, 0)).resolves.toBeInstanceOf(ArrayBuffer);
  });
});

describe("text archive commands", () => {
  beforeEach(() => invoke.mockReset());

  it("passes the edit as the backend's externally tagged enum", async () => {
    invoke.mockResolvedValue(7);
    await editTextArchive(433, { Set: { index: 1, text: "Neu" } });
    expect(invoke).toHaveBeenLastCalledWith("edit_text_archive", { index: 433, edit: { Set: { index: 1, text: "Neu" } } });
    await editTextArchive(433, "Append");
    expect(invoke).toHaveBeenLastCalledWith("edit_text_archive", { index: 433, edit: "Append" });
    await editTextArchive(433, "RemoveLast");
    expect(invoke).toHaveBeenLastCalledWith("edit_text_archive", { index: 433, edit: "RemoveLast" });
  });

  it("calls the read, create, search and check commands with their argument names", async () => {
    invoke.mockResolvedValue(null);
    await getTextArchive(5);
    expect(invoke).toHaveBeenLastCalledWith("get_text_archive", { index: 5 });
    await createTextArchive();
    expect(invoke).toHaveBeenLastCalledWith("create_text_archive");
    await searchText("hallo", 50);
    expect(invoke).toHaveBeenLastCalledWith("search_text", { query: "hallo", limit: 50 });
    await validateText("Hi");
    expect(invoke).toHaveBeenLastCalledWith("validate_text", { text: "Hi" });
  });
});

describe("language server commands", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("lspStart_hands_the_backend_a_fresh_channel_that_feeds_the_callback", async () => {
    invoke.mockResolvedValue(undefined);
    const first = vi.fn();
    const second = vi.fn();
    await lspStart(first);
    await lspStart(second);
    const [cmd, args] = invoke.mock.calls[0]!;
    expect(cmd).toBe("lsp_start");
    args.channel.onmessage('{"id":1}');
    expect(first).toHaveBeenCalledWith('{"id":1}');
    expect(invoke.mock.calls[1]![1].channel).not.toBe(args.channel);
  });

  it("lspStart_rejects_with_the_backend_error", async () => {
    const missing = { code: "tool_missing", message: "rotom-lsp fehlt — Completion und Diagnosen aus", asset: null };
    invoke.mockRejectedValue(missing);
    await expect(lspStart(vi.fn())).rejects.toBe(missing);
  });

  it("lspSend_and_lspStop_call_their_commands", async () => {
    invoke.mockResolvedValue(undefined);
    await lspSend("{}");
    await lspStop();
    expect(invoke.mock.calls).toEqual([["lsp_send", { body: "{}" }], ["lsp_stop"]]);
  });
});

describe("script commands", () => {
  beforeEach(() => invoke.mockReset());

  it("adoptScripts_passes_a_channel_that_feeds_the_progress_callback_and_resolves_to_the_report", async () => {
    const report = { decompiled: 2, skipped_empty: 0, failed: [] };
    invoke.mockResolvedValue(report);
    const onProgress = vi.fn();
    await expect(adoptScripts(onProgress)).resolves.toBe(report);
    const [cmd, args] = invoke.mock.calls[0]!;
    expect(cmd).toBe("adopt_scripts");
    args.progress.onmessage({ done: 1, total: 2 });
    expect(onProgress).toHaveBeenCalledWith({ done: 1, total: 2 });
  });

  it("calls_each_command_with_the_argument_names_of_the_backend", async () => {
    invoke.mockResolvedValue(undefined);
    await scriptsStatus();
    await readScript(3);
    await saveScript(3, "text");
    await saveLevelScript(4, { a: 1 });
    await addScript(3);
    await resolveScript(415, 2);
    expect(invoke.mock.calls).toEqual([
      ["scripts_status"],
      ["read_script", { file: 3 }],
      ["save_script", { file: 3, text: "text" }],
      ["save_level_script", { file: 4, doc: { a: 1 } }],
      ["add_script", { file: 3 }],
      ["resolve_script", { header: 415, n: 2 }],
    ]);
  });
});
