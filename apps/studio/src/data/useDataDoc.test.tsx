import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataList, encounterProblemFile, itemTable, nameTables } from "../api";
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
