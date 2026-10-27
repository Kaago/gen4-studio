import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const inTauri = vi.hoisted(() => vi.fn(() => true));
vi.mock("./platform/transport", () => ({ invoke, inTauri }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import {
  addBuilding,
  addEvent,
  addScript,
  adoptScripts,
  assignPlace,
  buildRom,
  createArea,
  createPlace,
  createRoom,
  createTextArchive,
  deleteBuilding,
  editTextArchive,
  getBuildingGlb,
  getTextArchive,
  getHeader,
  headerFamily,
  history,
  memberCounts,
  headerFields,
  isCmdError,
  placeSlots,
  renamePlace,
  lspSend,
  lspStart,
  lspStop,
  readScript,
  resolveScript,
  resolveTextConflicts,
  saveLevelScript,
  saveScript,
  scriptsStatus,
  searchText,
  setHeader,
  splitRoom,
  textConflictArchives,
  updateBuilding,
  validateText,
  worldMetaGet,
  worldMetaSet,
  type AreaReport,
  type AreaSpec,
  type BuildReport,
  type CmdError,
  type MapHeader,
  type MapHeaderHGSS,
  type MapHeaderPt,
  type RoomSpec,
  type SplitReport,
} from "./api";

describe("isCmdError", () => {
  it("recognises structured command errors", () => {
    expect(isCmdError({ code: "stale", message: "m", asset: { kind: "Map", index: 0 } })).toBe(true);
    expect(isCmdError({ code: "error", message: "m", asset: null })).toBe(true);
    expect(isCmdError({ code: "read_only", message: "m", asset: { kind: "Event", index: 3 } })).toBe(true);
    expect(isCmdError({ code: "partial", message: "map 0003 was created; header edit failed: boom", asset: null })).toBe(true);
    expect(isCmdError({ code: "finding_stale", message: "finding is no longer current", asset: null })).toBe(true);
    expect(isCmdError("plain string")).toBe(false);
  });

  it("accepts the compile code with its problems", () => {
    const problem = { severity: "error", file: "expanded/scripts/0000.rotom", line: 2, col: 5, message: "m", source: "compile" };
    const error = { code: "compile", message: "1 Fehler in Scripts oder Texten", asset: null, problems: [problem] };
    expect(isCmdError(error)).toBe(true);
    expect(isCmdError({ ...error, problems: [] })).toBe(true);
    expect(isCmdError({ ...error, problems: "none" })).toBe(false);
  });

  it("accepts the text_conflict code with its problems", () => {
    const problem = { severity: "error", file: "expanded/textArchives/0012.json", line: 0, col: 0, message: "m", source: "build" };
    expect(isCmdError({ code: "text_conflict", message: "m", asset: null, problems: [problem] })).toBe(true);
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

describe("history", () => {
  beforeEach(() => invoke.mockReset());

  it("asks for the undo and redo labels and returns them as sent", async () => {
    invoke.mockResolvedValue({ undo: ["A", "B"], redo: ["C"] });
    expect(await history()).toEqual({ undo: ["A", "B"], redo: ["C"] });
    expect(invoke).toHaveBeenCalledWith("history");
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

describe("event commands", () => {
  beforeEach(() => invoke.mockReset());

  it("add_event sends the patch and label only when given", async () => {
    invoke.mockResolvedValue(2);
    expect(await addEvent(7, "object", 9, 5)).toBe(2);
    expect(invoke).toHaveBeenLastCalledWith("add_event", { eventFile: 7, kind: "object", x: 9, z: 5 });
    await addEvent(7, "object", 9, 5, { script: 3929, trainer_type: 1, sight: 3 }, "Trainer Jens platziert");
    expect(invoke).toHaveBeenLastCalledWith("add_event", { eventFile: 7, kind: "object", x: 9, z: 5, patch: { script: 3929, trainer_type: 1, sight: 3 }, label: "Trainer Jens platziert" });
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
    inTauri.mockReturnValue(true);
  });

  it("lspStart_outside_tauri_rejects_with_tool_missing_without_a_channel_or_a_call", async () => {
    inTauri.mockReturnValue(false);
    await expect(lspStart(vi.fn())).rejects.toEqual({
      code: "tool_missing",
      message: "rotom-lsp fehlt — Completion und Diagnosen aus",
      asset: null,
    });
    expect(invoke).not.toHaveBeenCalled();
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
  beforeEach(() => {
    invoke.mockReset();
    inTauri.mockReturnValue(true);
  });

  it("adoptScripts_outside_tauri_calls_the_bridge_without_a_channel_and_reports_one_final_progress", async () => {
    inTauri.mockReturnValue(false);
    const report = { decompiled: 2, skipped_empty: 1, failed: [[7, "boom"]] };
    invoke.mockResolvedValue(report);
    const onProgress = vi.fn();
    await expect(adoptScripts(onProgress)).resolves.toBe(report);
    expect(invoke.mock.calls).toEqual([["adopt_scripts"]]);
    expect(onProgress.mock.calls).toEqual([[{ done: 4, total: 4 }]]);
  });

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

  it("worldMetaGet_and_worldMetaSet_call_the_world_commands_with_the_meta", async () => {
    invoke.mockReset();
    const meta = { version: 1 as const, worldMaps: [], notWorldMaps: [], roomNames: { "12": "Haus" }, roomOwner: {} };
    invoke.mockResolvedValueOnce({ meta, broken: null });
    await expect(worldMetaGet()).resolves.toEqual({ meta, broken: null });
    await worldMetaSet(meta);
    expect(invoke.mock.calls).toEqual([["world_meta_get"], ["world_meta_set", { meta }]]);
  });

  it("the_place_wrappers_send_the_keys_of_the_place_commands", async () => {
    invoke.mockReset();
    const slots = { bank: 433, count: 3, free: 253, reusable: [] };
    invoke.mockResolvedValueOnce(slots);
    await expect(placeSlots()).resolves.toEqual(slots);
    invoke.mockResolvedValueOnce({ index: 3, reused: null });
    await expect(createPlace("Neuheim", [0, 1])).resolves.toEqual({ index: 3, reused: null });
    await renamePlace(3, "Zweiblattdorf");
    await assignPlace([2], 3);
    expect(invoke.mock.calls).toEqual([
      ["place_slots"],
      ["create_place", { name: "Neuheim", headers: [0, 1] }],
      ["rename_place", { index: 3, name: "Zweiblattdorf" }],
      ["assign_place", { headers: [2], index: 3 }],
    ]);
  });

  it("the_wizard_wrappers_send_the_patch_flag_the_place_and_a_missing_internal_name", async () => {
    invoke.mockReset();
    const header = { id: 2, area_data_id: null, event_file: "New" as const, new_header: { template: 1, internal_name: null }, allow_patch: true, place: { New: "Neuheim" } };
    const area: AreaSpec = { name: "Neuheim", width: 1, height: 1, cells: [{ x: 0, y: 0, source_map: 0, clear_buildings: true }], with_headers: true, header };
    const room: RoomSpec = { name: "Haus", source_map: 0, clear_buildings: true, header: { ...header, id: 3, place: { Existing: 3 } }, entrance: null };
    const report: AreaReport = { matrix: 1, maps: [[0, 0, 3]], event_file: 2, header: 2, patched: true, snapshot: null, place: 3, replaced: 0 };
    invoke.mockResolvedValueOnce(report);
    await expect(createArea(area)).resolves.toEqual(report);
    await createRoom(room);
    expect(invoke.mock.calls).toEqual([
      ["create_area", { spec: area }],
      ["create_room", { spec: room }],
    ]);
  });

  it("create_area_sends_the_target_on_a_world_map_and_answers_the_replaced_cells", async () => {
    invoke.mockReset();
    const area: AreaSpec = {
      name: "Neuheim",
      width: 2,
      height: 1,
      cells: [{ x: 0, y: 0, source_map: 0, clear_buildings: true }, { x: 1, y: 0, source_map: 1, clear_buildings: true }],
      with_headers: false,
      header: null,
      target: { matrix: 0, x: 4, y: 2 },
    };
    const report: AreaReport = { matrix: 0, maps: [[0, 0, 3], [1, 0, 4]], event_file: null, header: null, patched: false, snapshot: null, place: null, replaced: 1 };
    invoke.mockResolvedValueOnce(report);
    await expect(createArea(area)).resolves.toEqual(report);
    expect(invoke.mock.calls).toEqual([["create_area", { spec: area }]]);
  });

  it("split_room_sends_the_header_and_the_name_or_null", async () => {
    invoke.mockReset();
    const report: SplitReport = { matrix: 4, maps: [9, 10] };
    invoke.mockResolvedValueOnce(report);
    await expect(splitRoom(7, "Haus")).resolves.toEqual(report);
    await splitRoom(8);
    expect(invoke.mock.calls).toEqual([
      ["split_room", { header: 7, name: "Haus" }],
      ["split_room", { header: 8, name: null }],
    ]);
  });
});

describe("build commands", () => {
  beforeEach(() => invoke.mockReset());

  it("buildRom_sends_fresh", async () => {
    const report: BuildReport = {
      saved: 0,
      packed: [],
      rom: "/p/out.nds",
      compiled: [],
      encoded: [],
      problems: [],
      steps: [{ id: "rom", status: "skipped", ms: 0, done: 0, total: 0, cached: 0, reason: null }],
      ms: 12,
    };
    invoke.mockResolvedValue(report);
    await expect(buildRom("/p/out.nds")).resolves.toEqual(report);
    await buildRom("/p/out.nds", true);
    expect(invoke.mock.calls).toEqual([
      ["build_rom", { out: "/p/out.nds", fresh: false }],
      ["build_rom", { out: "/p/out.nds", fresh: true }],
    ]);
  });

  it("resolveTextConflicts_sends_the_choices", async () => {
    invoke.mockResolvedValue([12, 433]);
    const choices = [
      { archive: 12, keep: "json" as const },
      { archive: 433, keep: "member" as const },
    ];
    await expect(resolveTextConflicts(choices)).resolves.toEqual([12, 433]);
    expect(invoke).toHaveBeenCalledWith("resolve_text_conflicts", { choices });
  });

  it("textConflictArchives_reads_the_problem_files", () => {
    const problem = (file: string) => ({ severity: "error" as const, file, line: 0, col: 0, message: "m", source: "build" as const });
    const error: CmdError = {
      code: "text_conflict",
      message: "m",
      asset: null,
      problems: [problem("expanded/textArchives/0012.json"), problem("expanded/textArchives/0433.json"), problem("files/.DS_Store")],
    };
    expect(textConflictArchives(error)).toEqual([12, 433]);
    expect(textConflictArchives({ ...error, problems: undefined })).toEqual([]);
  });
});
