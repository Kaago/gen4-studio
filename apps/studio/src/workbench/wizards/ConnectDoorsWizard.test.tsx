// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, recipeApply, recipePlan } from "../../api";
import type { ConnectDoorsReport, DraftDto, EventsDto, HeaderSummary, MatrixSummary, RecipeOutcome } from "../../api";
import { ConnectDoorsWizard } from "./ConnectDoorsWizard";
import { DRAFT_DEBOUNCE_MS } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), recipePlan: vi.fn(), recipeApply: vi.fn(), getEvents: vi.fn() }));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));

const header = (id: number, internal: string, location: string, matrix: number, event: number): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: location, matrix_id: matrix, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(0, "T01", "Zweiblattdorf", 0, 5), header(4, "T01R0201", "Spielerhaus", 4, 9)];
const matrices: MatrixSummary[] = [
  { index: 0, name: "m0", width: 2, height: 1, has_headers: true, has_altitudes: false, filled: 2 },
  { index: 4, name: "m4", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 },
];
const events = (file: number): EventsDto => ({
  event_file_id: file,
  locked: null,
  bg: [],
  objects: [],
  coords: [],
  warps:
    file === 5
      ? [
          { index: 0, x: 1, z: 1, dest_header_id: 9, dest_warp_id: 0, height: 0 },
          { index: 2, x: 7, z: 11, dest_header_id: 4, dest_warp_id: 0, height: 0 },
        ]
      : [{ index: 0, x: 3, z: 8, dest_header_id: 0, dest_warp_id: 2, height: 0 }],
});
const planned: DraftDto = {
  cells: [],
  tiles: [{ matrix: 4, x: 5, z: 6 }],
  events: [
    { kind: "warp", matrix: 0, x: 7, z: 11, look: null, target: { header: 4, warp: 1 } },
    { kind: "warp", matrix: 4, x: 5, z: 6, look: null, target: { header: 0, warp: 2 } },
  ],
  messages: [],
  notes: ["Tür 2 in Header 0 führt zu Tür 1 in Header 4.", "Tür 1 in Header 4 entsteht auf Kachel 5, 6."],
  irreversible: [],
};
const outcome: RecipeOutcome<ConnectDoorsReport> = {
  report: {
    a: { header: 0, event_file: 5, warp: 2, created: false, door_tile: null },
    b: { header: 4, event_file: 9, warp: 1, created: true, door_tile: { map: 321, x: 5, y: 6 } },
    back: true,
  },
  created: [],
  change: 21,
  changed: [
    { kind: "Event", index: 5 },
    { kind: "Event", index: 9 },
    { kind: "Map", index: 321 },
  ],
  next: [{ id: "play_here", label: "Hier spielen", available: false, reason: "Kommt mit 3d.9" }],
};

describe("ConnectDoorsWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  const onDone = vi.fn();
  const onCancel = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getEvents).mockReset().mockImplementation(async (file) => events(file));
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    vi.mocked(recipeApply).mockReset().mockResolvedValue(outcome);
    onDraft.mockReset();
    onDone.mockReset().mockResolvedValue(undefined);
    onCancel.mockReset().mockImplementation(() => root.render(null));
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  async function show(prefill: { a?: { header: number; warp?: number; tile?: { x: number; z: number } } }) {
    await act(async () =>
      root.render(<ConnectDoorsWizard depth="erzaehlen" headers={headers} matrices={matrices} prefill={prefill} onDraft={onDraft} onDone={onDone} onCancel={onCancel} onError={onError} />),
    );
    await flush();
  }
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const nextDisabled = () => button("Weiter")?.hasAttribute("disabled") === true;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  const end = (which: "a" | "b") => container.querySelector<HTMLElement>(`[data-door-end="${which}"]`)!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const checkbox = (label: string) => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((c) => c.closest("label")?.textContent?.startsWith(label));
  async function pick(opener: string, text: string, scope: ParentNode = container) {
    await click(button(opener, scope));
    await flush();
    await click(options().find((o) => o.textContent?.includes(text)));
    await flush();
  }
  async function typeTile(scope: HTMLElement, x: number, z: number) {
    await act(async () => {
      for (const [label, value] of [
        ["Kachel x", x],
        ["Kachel z", z],
      ] as const) {
        const input = scope.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
        input.focus();
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, String(value));
        input.dispatchEvent(new Event("input", { bubbles: true }));
        input.blur();
      }
    });
  }

  it("two_ends_door_or_tile_with_back_and_door_tiles", async () => {
    await show({ a: { header: 0 } });
    expect(container.querySelector('[data-wizard="connect_doors"]')).not.toBeNull();
    expect(current()).toBe("1. Tür A");
    expect(end("a").textContent).toContain("Zweiblattdorf");
    // An existing door of the place, picked by its number and tile as the door field does.
    expect(button("Vorhandene Tür", end("a"))?.getAttribute("aria-pressed")).toBe("true");
    expect(nextDisabled()).toBe(true);
    await pick("Tür wählen…", "Tür 2 · x 7, z 11", end("a"));
    expect(end("a").textContent).toContain("Tür 2 · x 7, z 11");
    await next();
    expect(current()).toBe("2. Tür B");
    // Door B: another place, a new tile there.
    await pick("Ort wählen…", "Spielerhaus", end("b"));
    await click(button("Neue Kachel", end("b")));
    await typeTile(end("b"), 5, 6);
    expect(checkbox("Rückweg anlegen")?.getAttribute("aria-checked")).toBe("true");
    expect(checkbox("Türkacheln setzen")?.getAttribute("aria-checked")).toBe("true");
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("connect_doors", { a: { header: 0, warp: 2 }, b: { header: 4, tile: { x: 5, z: 6 } }, back: true, door_tiles: true });
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    // Without the way back door B must be a door that is there (the kernel refuses a new one): the wizard says so and waits.
    await click(checkbox("Rückweg anlegen"));
    expect(container.textContent).toContain("Ohne Rückweg muss Tür B eine vorhandene Tür sein");
    expect(nextDisabled()).toBe(true);
    await click(checkbox("Rückweg anlegen"));
    await click(checkbox("Türkacheln setzen"));
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("connect_doors", { a: { header: 0, warp: 2 }, b: { header: 4, tile: { x: 5, z: 6 } }, back: true, door_tiles: false });
    await click(checkbox("Türkacheln setzen"));
    await next();
    expect(current()).toBe("3. Zusammenfassung");
    expect(container.textContent).toContain("Tür A: Tür 2 in Zweiblattdorf");
    expect(container.textContent).toContain("Tür B: neue Tür auf Kachel x 5, z 6 in Spielerhaus");
    await click(button("Verbinden"));
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("connect_doors", { a: { header: 0, warp: 2 }, b: { header: 4, tile: { x: 5, z: 6 } }, back: true, door_tiles: true });
    expect(onDone).toHaveBeenCalledOnce();
    expect(container.textContent).toContain("Türen verbunden");
    expect(container.textContent).toContain("Tür A: Warp 2 in Eventdatei 0005");
    expect(container.textContent).toContain("Tür B: Warp 1 in Eventdatei 0009 (neu)");
    const steps = [...container.querySelectorAll<HTMLButtonElement>("[data-next-steps] button[data-recipe]")];
    expect(steps.map((b) => [b.getAttribute("data-recipe"), b.disabled])).toEqual([["play_here", true]]);
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });

  it("prefill_a_from_the_selected_door", async () => {
    // „Tür verbinden…“ in the door's inspector: door A is that door, the wizard asks only for door B.
    await show({ a: { header: 0, warp: 2 } });
    expect(end("a").textContent).toContain("Zweiblattdorf");
    expect(end("a").textContent).toContain("Tür 2 · x 7, z 11");
    expect(nextDisabled()).toBe(false);
    await next();
    // Door B: an existing door of the house, the way back on.
    await pick("Ort wählen…", "Spielerhaus", end("b"));
    await pick("Tür wählen…", "Tür 0 · x 3, z 8", end("b"));
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("connect_doors", { a: { header: 0, warp: 2 }, b: { header: 4, warp: 0 }, back: true, door_tiles: true });
    // A tile for door A from the scene: a new door there.
    await act(async () => root.unmount());
    root = createRoot(container);
    await show({ a: { header: 0, tile: { x: 12, z: 7 } } });
    expect(button("Neue Kachel", end("a"))?.getAttribute("aria-pressed")).toBe("true");
    expect(end("a").querySelector<HTMLInputElement>('input[aria-label="Kachel x"]')?.value).toBe("12");
  });
});
