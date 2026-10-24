import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataList, encounterProblemFile, itemTable, nameTables, trainerProblemFile } from "../api";
import type { DataDocDto, ProblemDto } from "../api";
import { ProblemsStore } from "../workbench/problems/problems";
import { NAMES, dataHarness, flush } from "./dataTesting";
import { PokemonArea } from "./PokemonArea";
import { useDataDoc } from "./useDataDoc";
import type { DocSpec } from "./useDataDoc";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type EncounterDoc = Extract<DataDocDto, { kind: "encounter" }>;

const warning = (index: number, line: number, message: string): ProblemDto => ({
  severity: "warning",
  file: encounterProblemFile(index),
  line,
  col: 1,
  message,
  source: "encounter",
});

const encounter = (problems: ProblemDto[]): EncounterDoc => ({
  kind: "encounter",
  methods: [],
  replacements: [],
  extras: { form_rates: [0, 0, 0, 0, 0], unown_table: 0 },
  warnings: [],
  problems,
  users: [],
  effective: [],
  locked: null,
  raw_locked: null,
});

const SPEC: DocSpec<EncounterDoc> = {
  area: "pokemon",
  kind: "encounter",
  parts: ["encounter"],
  accept: (doc) => (doc.kind === "encounter" ? doc : null),
  wrongKind: "keine Begegnungsdatei",
  problems: (doc, index) => ({ file: encounterProblemFile(index), rows: doc.problems }),
};

let update: ((index: number) => Promise<boolean>) | null = null;

function Probe({ index }: { index: number }) {
  const doc = useDataDoc(SPEC, index);
  update = (i) => doc.update(i, () => ({ patch: { Encounter: { method: "grass", slot: 0, field: "species", value: 2 } }, label: "Route 201: Gras Platz 1" }));
  return null;
}

describe("useDataDoc with Problems rows", () => {
  let backend: EncounterDoc;
  beforeEach(() => {
    backend = encounter([warning(12, 1, "swarm entry 0 is empty"), warning(12, 3, "grass slot 2: level 0")]);
    vi.mocked(dataList).mockReset().mockResolvedValue([]);
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    vi.mocked(itemTable).mockReset().mockResolvedValue(null);
    vi.mocked(dataGet).mockReset().mockImplementation(async () => backend);
    vi.mocked(dataEdit).mockReset().mockImplementation(async () => {
      backend = encounter([warning(12, 3, "grass slot 2: level 0")]);
      return [{ kind: "Encounter", index: 12 }];
    });
  });
  afterEach(() => {
    update = null;
  });

  it("replaces the member's rows with every read: after data_get and after data_edit", async () => {
    const problems = new ProblemsStore();
    const h = dataHarness((props) => (
      <PokemonArea context={props}>
        <Probe index={12} />
      </PokemonArea>
    ));
    h.show(h.base({ problems }));
    await flush();
    expect(problems.list().map((p) => `${p.source}:${p.file}:${p.line}`)).toEqual(["encounter:encounters/0012:1", "encounter:encounters/0012:3"]);
    await act(async () => {
      await update?.(12);
    });
    await flush();
    // The edit announced the member; the read after it replaced the rows instead of adding to them.
    expect(problems.list().map((p) => p.message)).toEqual(["grass slot 2: level 0"]);
    h.unmount();
  });
});

type TrainerDoc = Extract<DataDocDto, { kind: "trainer" }>;

const trainerDoc = (text: string, problems: ProblemDto[]): TrainerDoc => ({
  kind: "trainer",
  class: 1,
  name: "Jens",
  format: { moves: false, items: false },
  ai: 1,
  battle_type: 0,
  items: [0, 0, 0, 0],
  party: [],
  texts: [{ kind: 0, archive: 617, message: 2, text }],
  texts_locked: null,
  events: [],
  name_archive: 618,
  text_archive: 617,
  problems,
  placeholder: false,
  locked: null,
  raw_locked: null,
});

const TRAINER_SPEC: DocSpec<TrainerDoc> = {
  area: "pokemon",
  kind: "trainer",
  parts: ["trainer"],
  accept: (doc) => (doc.kind === "trainer" ? doc : null),
  wrongKind: "kein Trainer",
  // The name and the battle texts live in text archives: a change there must read the trainer again.
  texts: (doc) => [doc.name_archive, doc.text_archive],
  problems: (doc, index) => ({ file: trainerProblemFile(index), rows: doc.problems }),
};

let fixText: (() => Promise<boolean>) | null = null;
let shown: TrainerDoc | null = null;

function TrainerProbe() {
  const doc = useDataDoc(TRAINER_SPEC, 928);
  shown = doc.doc;
  fixText = () => doc.update(928, () => ({ patch: { TrainerTexts: { texts: [{ kind: 0, text: "Los!\\r" }] } }, label: "Wanderer Jens: Kampftext" }));
  return null;
}

describe("useDataDoc with a text-only change", () => {
  it("reads the trainer again when the edit changed only a text archive, and its Problems row goes", async () => {
    const row: ProblemDto = { severity: "warning", file: trainerProblemFile(928), line: 1, col: 1, message: "trainer 928: no \\r", source: "trainer" };
    let backend = trainerDoc("Los!", [row]);
    vi.mocked(dataList).mockReset().mockResolvedValue([]);
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    vi.mocked(itemTable).mockReset().mockResolvedValue(null);
    vi.mocked(dataGet).mockReset().mockImplementation(async () => backend);
    // The backend changed the battle text archive alone (the kind existed): no data member.
    vi.mocked(dataEdit).mockReset().mockImplementation(async () => {
      backend = trainerDoc("Los!\\r", []);
      return [{ kind: "TextArchive", index: 617 }];
    });
    const problems = new ProblemsStore();
    const h = dataHarness((props) => (
      <PokemonArea context={props}>
        <TrainerProbe />
      </PokemonArea>
    ));
    const events: unknown[] = [];
    h.bus.subscribe(
      () => true,
      (event) => void events.push(event),
    );
    h.show(h.base({ problems }));
    await flush();
    expect(problems.list().map((p) => `${p.source}:${p.file}`)).toEqual(["trainer:trainers/0928"]);
    const reads = vi.mocked(dataGet).mock.calls.length;
    await act(async () => {
      await fixText?.();
    });
    await flush();
    expect(events).toContainEqual({ topic: "textData", archive: 617 });
    // Read again beyond the read the edit makes to build its patch: the text and the rows follow the backend.
    expect(vi.mocked(dataGet).mock.calls.length).toBeGreaterThanOrEqual(reads + 2);
    expect(shown?.texts[0]?.text).toBe("Los!\\r");
    expect(problems.list()).toEqual([]);
    h.unmount();
  });
});
