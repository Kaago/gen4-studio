// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import { createEditBus } from "../bus/editBus";
import { DataStore } from "../data/dataStore";
import { useDataArea } from "../data/useDataArea";
import type { DataAreaModel } from "../data/useDataArea";
import { ShellProvider } from "../ui/frame/ShellContext";
import { DataAreaHost } from "./panels";
import { ProblemsStore } from "./problems/problems";
import { NO_WORLD, WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData, WizardRequest } from "./WorkbenchContext";

// The scene needs WebGL: it is not under test here.
vi.mock("../world/MatrixScene", () => ({ default: () => null }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string, matrix_id = 0): HeaderSummary => ({ id, internal_name, location_index: 0, kanto: false, location_name: "", matrix_id, area_data_id: 0, event_file_id: id + 100, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const HEADERS = [header(3, "T01"), header(5, "R201"), header(9, "T02", 1)];

describe("the data areas open the world's wizards", () => {
  let container: HTMLElement;
  let root: Root;
  let area: DataAreaModel | null = null;
  const onOpenWizard = vi.fn();
  const onSelect = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    localStorage.clear();
    area = null;
    onOpenWizard.mockReset();
    onSelect.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function Probe() {
    area = useDataArea("pokemon");
    return null;
  }

  const show = async (selected: number | null) => {
    const data = {
      bus: createEditBus(),
      info: null,
      headers: HEADERS,
      selected,
      onSelect,
      onOpenWizard,
      goTo: vi.fn(),
      dirtyAssets: [],
      onEdited: vi.fn(),
      problems: new ProblemsStore(),
      scripts: { saveIfDirty: vi.fn(), reload: vi.fn() },
      depth: "erzaehlen",
    } as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world: NO_WORLD }}>
            <DataAreaHost store={new DataStore()} target={null} onTrail={vi.fn()}>
              <Probe />
            </DataAreaHost>
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const open = (request: WizardRequest) => act(() => area!.openWizard!(request));

  it("shows_the_place_of_neuer_trainer_in_the_world_when_the_world_shows_another", async () => {
    await show(3);
    await open({ kind: "new_trainer_here", prefill: { header: 5 } });
    expect(onOpenWizard).toHaveBeenCalledExactlyOnceWith({ kind: "new_trainer_here", prefill: { header: 5 } });
    // The world selects the place as a click in its list does: the scene shows it with the wizard's draft.
    expect(onSelect).toHaveBeenCalledExactlyOnceWith(HEADERS[1]);
    // The wizard opens first, so it remembers Pokémon as the area to go back to.
    expect(onOpenWizard.mock.invocationCallOrder[0]).toBeLessThan(onSelect.mock.invocationCallOrder[0]);
  });

  it("shows_the_place_when_the_world_shows_none_yet", async () => {
    await show(null);
    await open({ kind: "new_trainer_here", prefill: { header: 9 } });
    expect(onSelect).toHaveBeenCalledExactlyOnceWith(HEADERS[2]);
  });

  it("leaves_the_world_alone_when_it_shows_the_place_already_or_there_is_none", async () => {
    await show(5);
    await open({ kind: "new_trainer_here", prefill: { header: 5 } });
    await open({ kind: "new_trainer_here" });
    // A header the list does not know is not selected (the wizard says the place is missing).
    await open({ kind: "new_trainer_here", prefill: { header: 77 } });
    expect(onOpenWizard).toHaveBeenCalledTimes(3);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
