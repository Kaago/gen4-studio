import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import { draftCount, recipeApply, recipeList, recipePlan } from "../api";
import type {
  ConnectDoorsInput,
  ConnectDoorsReport,
  DraftDto,
  FigureAppearsInput,
  FigureAppearsReport,
  NewFigureInput,
  NewFigureReport,
  NewStoryInput,
  NewStoryReport,
  NewTrainerHereInput,
  NewTrainerHereReport,
  RecipeId,
  RecipeInfo,
  RecipeOutcome,
  RewardOnceInput,
  RewardOnceReport,
  SceneOnEnterInput,
  SceneOnEnterReport,
  TalkAdvancesInput,
  TalkAdvancesReport,
} from "../api";
import { RECIPE_TITLES } from "../api";

const empty: DraftDto = { cells: [], tiles: [], events: [], messages: [], notes: [], irreversible: [] };

describe("recipes/api", () => {
  beforeEach(() => invoke.mockReset());

  it("calls_the_recipe_commands_with_recipe_and_input", async () => {
    const list: RecipeInfo[] = [{ id: "empty_region", title: "Welt leeren", irreversible: true }];
    invoke.mockResolvedValueOnce(list);
    await expect(recipeList()).resolves.toBe(list);
    invoke.mockResolvedValueOnce(empty);
    const input = { matrix: 0, x: 2, y: 0, source_map: 1, clear_buildings: false, header_cell: null };
    await expect(recipePlan("extend_area", input)).resolves.toBe(empty);
    const outcome: RecipeOutcome<{ map: number }> = {
      report: { map: 3 },
      created: [{ kind: "Map", index: 3 }],
      change: 7,
      changed: [{ kind: "Matrix", index: 0 }],
      next: [{ id: "adjust_terrain", label: "Gelände anpassen", available: false, reason: "Kommt mit Phase 6" }],
    };
    invoke.mockResolvedValueOnce(outcome);
    await expect(recipeApply<{ map: number }>("extend_area", input)).resolves.toBe(outcome);
    expect(invoke.mock.calls).toEqual([
      ["recipe_list"],
      ["recipe_plan", { recipe: "extend_area", input }],
      ["recipe_apply", { recipe: "extend_area", input }],
    ]);
  });

  it("recipe_ids_name_twelve", () => {
    // `satisfies` makes tsc (pnpm build) refuse a missing or an unknown id: the object names every `RecipeId` once.
    const every = {
      create_area: true,
      create_room: true,
      extend_area: true,
      empty_region: true,
      new_figure: true,
      connect_doors: true,
      new_trainer_here: true,
      new_story: true,
      talk_advances: true,
      figure_appears: true,
      scene_on_enter: true,
      reward_once: true,
      new_painted_map: true,
    } satisfies Record<RecipeId, true>;
    expect(Object.keys(every)).toEqual(Object.keys(RECIPE_TITLES));
    // The kernel's `RecipeId::title`, in the order of `all()`.
    expect(Object.values(RECIPE_TITLES)).toEqual([
      "Gebiet anlegen",
      "Raum anlegen",
      "Gebiet erweitern",
      "Welt leeren",
      "Neue Figur",
      "Tür verbinden",
      "Neuer Trainer hier",
      "Neue Geschichte",
      "Gespräch schaltet weiter",
      "Figur erscheint/verschwindet ab Schritt",
      "Szene beim Betreten",
      "Belohnung einmalig",
      "Neue Karte malen",
    ]);
  });

  it("the_inputs_and_reports_of_the_five_story_recipes_carry_the_rust_names", async () => {
    invoke.mockResolvedValue(empty);
    const figure = { header: 411, local_id: 3 };
    const inputs: [RecipeId, unknown][] = [
      ["new_story", { name: "Rivalin", var: 0x406c, notes: "" } satisfies NewStoryInput],
      ["talk_advances", { story: "STORY_RIVALIN", figure, from_step: 0, text: "Hallo!", step_name: null } satisfies TalkAdvancesInput],
      ["figure_appears", { story: "STORY_RIVALIN", figure, step: 1, appears: true, flag: 0x173, step_name: null } satisfies FigureAppearsInput],
      ["scene_on_enter", { story: "STORY_RIVALIN", header: 412, step: 1, text: "Hier!", step_name: "Szene beim Betreten von Zweiblattdorf" } satisfies SceneOnEnterInput],
      ["reward_once", { story: "STORY_RIVALIN", figure, from_step: 2, item: 17, text: "Für dich.", flag: 0x174, step_name: "Belohnung: Trank" } satisfies RewardOnceInput],
    ];
    for (const [recipe, input] of inputs) await recipePlan(recipe, input);
    expect(invoke.mock.calls.map(([name, args]) => [name, args.recipe, Object.keys(args.input)])).toEqual([
      ["recipe_plan", "new_story", ["name", "var", "notes"]],
      ["recipe_plan", "talk_advances", ["story", "figure", "from_step", "text", "step_name"]],
      ["recipe_plan", "figure_appears", ["story", "figure", "step", "appears", "flag", "step_name"]],
      ["recipe_plan", "scene_on_enter", ["story", "header", "step", "text", "step_name"]],
      ["recipe_plan", "reward_once", ["story", "figure", "from_step", "item", "text", "flag", "step_name"]],
    ]);
    const reports: [NewStoryReport, TalkAdvancesReport, FigureAppearsReport, SceneOnEnterReport, RewardOnceReport] = [
      { story: "STORY_RIVALIN", number: 0, var: 0x406c, var_id: "VAR_STORY_RIVALIN", files: ["data/ids/vars.json"] },
      { step: "s1", value: 1, event_file: 5, object: 2, script_file: 7, slot: 4, previous_slot: 1, archive: 554, message: 12 },
      { step: "a371", flag: 0x173, flag_id: "FLAG_STORY_RIVALIN_SICHTBAR_3", event_file: 5, object: 2, level_script: 9, level_script_created: false, script_file: 7, slot: 5, rules: 1 },
      { step: "s2", value: 2, level_script: 9, level_script_created: true, condition: 0, script_file: 7, slot: 6, archive: 554, message: 13 },
      { step: "r372", flag: 0x174, flag_id: "FLAG_STORY_RIVALIN_BELOHNUNG_3", event_file: 5, object: 2, script_file: 7, slot: 7, previous_slot: 4, archive: 554, message: 14 },
    ];
    expect(reports.map((r) => Object.keys(r).length)).toEqual([5, 9, 10, 9, 10]);
  });

  it("the_inputs_of_the_three_recipes_go_as_written", async () => {
    invoke.mockResolvedValue(empty);
    const figure: NewFigureInput = { header: 411, x: 10, z: 12, graphics_id: null, movement_type: 0, dir: 0, text: "Hallo!", hidden_flag: null };
    // An end is a door that is there (warp) or a tile that gets one (tile); back and door_tiles default to true.
    const doors: ConnectDoorsInput = { a: { header: 411, warp: 0 }, b: { header: 412, tile: { x: 5, z: 9 } }, back: true, door_tiles: true };
    const trainer: NewTrainerHereInput = {
      header: 411,
      x: 8,
      z: 8,
      dir: 1,
      sight: 3,
      class: 2,
      name: "Jens",
      party: [{ species: 396, level: 5 }],
      texts: { before: "Los geht's!\r", lose: "Schade.", after: "Gut gekämpft." },
      graphics_id: null,
    };
    await recipePlan("new_figure", figure);
    await recipePlan("connect_doors", { a: { header: 411, warp: 0 }, b: { header: 412, warp: 1 } } satisfies ConnectDoorsInput);
    await recipePlan("connect_doors", doors);
    await recipePlan("new_trainer_here", trainer);
    expect(invoke.mock.calls.map(([name, args]) => [name, args.recipe])).toEqual([
      ["recipe_plan", "new_figure"],
      ["recipe_plan", "connect_doors"],
      ["recipe_plan", "connect_doors"],
      ["recipe_plan", "new_trainer_here"],
    ]);
    expect(invoke.mock.calls[2][1].input).toBe(doors);

    const reports: [NewFigureReport, ConnectDoorsReport, NewTrainerHereReport] = [
      { event_file: 5, object: 2, local_id: 3, script_file: 7, slot: 4, archive: 554, message: 12 },
      {
        a: { header: 411, event_file: 5, warp: 0, created: false, door_tile: null },
        b: { header: 412, event_file: 6, warp: 1, created: true, door_tile: { map: 33, x: 5, y: 9 } },
        back: true,
      },
      {
        trainer: 929,
        placeholder: null,
        script_extended: false,
        event_file: 5,
        object: 3,
        script: 3928,
        trainer_flag: 1478,
        text_problems: [{ severity: "warning", file: "", line: 1, col: 1, message: "Zu breit." }],
      },
    ];
    expect(reports.map((r) => Object.keys(r).length)).toEqual([7, 3, 8]);
  });

  it("draft_count_counts_cells_tiles_and_events_of_the_matrix", () => {
    const draft: DraftDto = {
      ...empty,
      cells: [
        { matrix: 4, x: 0, y: 0, source_map: 9 },
        { matrix: 5, x: 1, y: 0, source_map: 9 },
        { matrix: null, x: 0, y: 0, source_map: 9 },
      ],
      tiles: [
        { matrix: 4, x: 10, z: 12 },
        { matrix: null, x: 3, z: 3 },
      ],
      events: [
        { kind: "warp", matrix: 4, x: 10, z: 13, look: null, target: { header: 2, warp: null } },
        { kind: "object", matrix: null, x: 5, z: 5, look: 3, target: null },
      ],
      messages: [{ archive: 12, text: "Hallo" }],
      notes: ["Legt Raum an."],
    };
    expect(draftCount(draft, 4)).toBe(3);
    expect(draftCount(draft, 5)).toBe(1);
    expect(draftCount(draft, 6)).toBe(0);
    expect(draftCount(null, 4)).toBe(0);
  });
});
