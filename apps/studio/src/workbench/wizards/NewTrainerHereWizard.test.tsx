// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getBattleSprite, getMonIcon, getObjectSprite, getTrainerSprite, nameTables, recipeApply, recipePlan, validateText } from "../../api";
import type { DraftDto, HeaderSummary, MatrixSummary, NameTablesDto, NewTrainerHereReport, ProjectInfo, RecipeOutcome } from "../../api";
import { resetIcons } from "../../data/icons";
import { NewTrainerHereWizard } from "./NewTrainerHereWizard";
import { DRAFT_DEBOUNCE_MS } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  recipePlan: vi.fn(),
  recipeApply: vi.fn(),
  nameTables: vi.fn(),
  validateText: vi.fn(async () => []),
  getObjectSprite: vi.fn(async () => null),
  getMonIcon: vi.fn(),
  getBattleSprite: vi.fn(),
  getTrainerSprite: vi.fn(),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS + 250)));

const header = (id: number, internal: string, location: string, matrix: number, event: number): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: location, matrix_id: matrix, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(0, "T01", "Zweiblattdorf", 0, 5), header(4, "T01R0201", "Spielerhaus", 4, 9)];
const matrices: MatrixSummary[] = [{ index: 0, name: "m0", width: 2, height: 1, has_headers: true, has_altitudes: false, filled: 2 }];
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 5 };
const names: NameTablesDto = {
  species: ["-----", "Bisasam", "Bisaknosp", "Bisaflor", "Glumanda"],
  moves: [],
  items: [],
  abilities: [],
  types: [],
  classes: ["Pokémon-Trainer", "Wanderer", "Käfersammler"],
  forms: [],
  archives: { species: null, moves: null, items: null, abilities: null, types: null, classes: null, forms: null },
  sounds: [],
};
const planned: DraftDto = {
  cells: [],
  tiles: [],
  events: [{ kind: "object", matrix: 0, x: 12, z: 7, look: 20, target: null }],
  messages: [],
  notes: ["Legt Trainer 42 „Jens“ (Wanderer) mit 1 Pokémon an.", "Stellt Objekt 3 auf Kachel 12, 7 von Header 0, Sichtweite 3."],
  irreversible: ["Trainer 42 wird angelegt"],
};
const outcome: RecipeOutcome<NewTrainerHereReport> = {
  report: { trainer: 42, placeholder: null, script_extended: false, event_file: 5, object: 3, script: 3041, trainer_flag: 1407, text_problems: [] },
  created: [{ kind: "Trainer", index: 42 }],
  change: 30,
  changed: [
    { kind: "Trainer", index: 42 },
    { kind: "Event", index: 5 },
  ],
  next: [{ id: "play_here", label: "Hier spielen", available: false, reason: "Kommt mit 3d.9" }],
};

describe("NewTrainerHereWizard", () => {
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
    vi.mocked(nameTables).mockReset().mockResolvedValue(names);
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    vi.mocked(recipeApply).mockReset().mockResolvedValue(outcome);
    vi.mocked(validateText).mockClear();
    vi.mocked(getObjectSprite).mockClear();
    resetIcons();
    vi.mocked(getMonIcon).mockReset().mockResolvedValue(new ArrayBuffer(4));
    vi.mocked(getBattleSprite).mockReset().mockResolvedValue(new ArrayBuffer(4));
    vi.mocked(getTrainerSprite).mockReset().mockResolvedValue(new ArrayBuffer(4));
    URL.createObjectURL = vi.fn(() => "blob:icon");
    URL.revokeObjectURL = vi.fn();
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

  async function show() {
    await act(async () =>
      root.render(
        <NewTrainerHereWizard depth="erzaehlen" headers={headers} matrices={matrices} project={project} prefill={{ header: 0, tile: { x: 12, z: 7 } }} onDraft={onDraft} onDone={onDone} onCancel={onCancel} onError={onError} />,
      ),
    );
    await flush();
  }
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const buttons = (name: string) => [...container.querySelectorAll("button")].filter((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const nextDisabled = () => button("Weiter")?.hasAttribute("disabled") === true;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  const rows = () => [...container.querySelectorAll<HTMLElement>("[data-team-row]")];
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  async function setValue(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
    const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    await act(async () => {
      el.focus();
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(el, value);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => el.blur());
  }
  async function pickSpecies(row: HTMLElement, name: string) {
    await click(button("Art wählen…", row));
    await flush();
    await click(options().find((o) => o.textContent?.includes(name)));
    await flush();
  }
  /** Step „Trainer“: the class stays the first, the name is typed. */
  async function toTeam() {
    await show();
    await setValue(container.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jens");
    await next();
  }

  it("team_rows_one_to_six_and_the_before_text_hint", async () => {
    await show();
    expect(container.querySelector('[data-wizard="new_trainer_here"]')).not.toBeNull();
    expect(current()).toBe("1. Trainer");
    // A name longer than the game copies is refused with the kernel's bound.
    await setValue(container.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jens-Peter");
    expect(container.textContent).toContain("Höchstens 7 Zeichen");
    expect(nextDisabled()).toBe(true);
    await setValue(container.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jens");
    await next();
    expect(current()).toBe("2. Team");
    // One row to start with; it needs a species before the team is complete.
    expect(rows()).toHaveLength(1);
    expect(nextDisabled()).toBe(true);
    await pickSpecies(rows()[0], "Bisasam");
    expect(rows()[0].querySelector("[data-species]")?.getAttribute("data-species")).toBe("1");
    expect(rows()[0].textContent).toContain("Bisasam");
    expect(nextDisabled()).toBe(false);
    // The only row cannot go; up to six rows.
    expect(button("Pokémon 1 entfernen")?.hasAttribute("disabled")).toBe(true);
    for (let i = 0; i < 5; i++) await click(button("Pokémon hinzufügen"));
    expect(rows()).toHaveLength(6);
    expect(button("Pokémon hinzufügen")?.hasAttribute("disabled")).toBe(true);
    // New rows wait for their species.
    expect(nextDisabled()).toBe(true);
    await click(button("Pokémon 6 entfernen"));
    await click(button("Pokémon 5 entfernen"));
    await click(button("Pokémon 4 entfernen"));
    await click(button("Pokémon 3 entfernen"));
    expect(rows()).toHaveLength(2);
    await pickSpecies(rows()[1], "Glumanda");
    // The level, 1 to 100.
    await setValue(rows()[1].querySelector<HTMLInputElement>('input[aria-label="Level"]')!, "12");
    expect(rows()[1].textContent).toContain("Lv. 12");
    await next();
    expect(current()).toBe("3. Texte");
    // The text before the battle must end on ▼, else its box closes at once.
    expect(container.textContent).toContain("Der Text vor dem Kampf endet auf ▼");
    const before = container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Vor dem Kampf"]')!;
    await setValue(before, "Halt!");
    expect(nextDisabled()).toBe(true);
    await setValue(before, "Halt!\\r");
    expect(nextDisabled()).toBe(false);
    expect(buttons("Weiter")).toHaveLength(1);
  });

  it("summary_names_the_barrier", async () => {
    await toTeam();
    await pickSpecies(rows()[0], "Bisasam");
    await next();
    await setValue(container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Vor dem Kampf"]')!, "Halt!\\r");
    await setValue(container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Verloren"]')!, "Oh nein!");
    await next();
    expect(current()).toBe("4. Wo");
    expect(container.querySelector("[data-figure-place]")?.textContent).toContain("Zweiblattdorf");
    expect(container.querySelector<HTMLInputElement>('input[aria-label="Kachel x"]')?.value).toBe("12");
    expect(container.querySelector<HTMLInputElement>('input[aria-label="Sichtweite"]')?.value).toBe("3");
    await settle();
    const input = {
      header: 0,
      x: 12,
      z: 7,
      dir: 1,
      sight: 3,
      class: 0,
      name: "Jens",
      party: [{ species: 1, level: 5 }],
      texts: { before: "Halt!\\r", lose: "Oh nein!", after: "" },
      graphics_id: null,
    };
    expect(recipePlan).toHaveBeenLastCalledWith("new_trainer_here", input);
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await next();
    expect(current()).toBe("5. Zusammenfassung");
    // The barrier: the trainer stays after undo, said in the step and with the frame's badge.
    expect(container.querySelector("[data-trainer-barrier]")?.textContent).toContain("Trainer anlegen lässt sich nicht rückgängig machen");
    expect(container.textContent).toContain("Nicht rückgängig");
    expect(container.textContent).toContain("Jens (Pokémon-Trainer) mit Bisasam Lv. 5");
    await click(button("Aufstellen"));
    expect(recipeApply).toHaveBeenCalledExactlyOnceWith("new_trainer_here", input);
    expect(onDone).toHaveBeenCalledExactlyOnceWith(outcome, input);
    expect(container.textContent).toContain("Trainer aufgestellt");
    // The created trainer (it stays after undo) and where it stands.
    expect(container.textContent).toContain("Trainer 0042 angelegt (bleibt nach Rückgängig)");
    expect(container.textContent).toContain("Objekt 3 in Eventdatei 0005, Kampf-Script 3041");
    expect(container.textContent).toContain("Kampfvorschau: Pokémon-Trainer Jens gegen Bisasam Lv. 5");
    const steps = [...container.querySelectorAll<HTMLButtonElement>("[data-next-steps] button[data-recipe]")];
    expect(steps.map((b) => [b.getAttribute("data-recipe"), b.disabled])).toEqual([["play_here", true]]);
  });

  it("a_failure_after_the_trainer_shows_what_stayed", async () => {
    vi.mocked(recipeApply).mockRejectedValueOnce({ code: "partial", message: "trainer 0042 was created; placing the object failed: Eventdatei 0005 ist gesperrt", asset: null });
    await toTeam();
    await pickSpecies(rows()[0], "Bisasam");
    await next();
    await setValue(container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Vor dem Kampf"]')!, "Halt!\\r");
    await next();
    await next();
    await click(button("Aufstellen"));
    expect(container.textContent).toContain("Anlegen unterbrochen");
    expect(container.textContent).toContain("Eventdatei 0005 ist gesperrt");
    // The trainer is there: the app reads the project again when the wizard is left.
    await click(button("Schließen"));
    expect(onCancel).toHaveBeenCalledWith(true);
  });

  it("the preview follows the form", async () => {
    await show();
    const preview = () => container.querySelector<HTMLElement>("[data-trainer-preview]");
    const foe = () => preview()?.querySelector('[data-ds-status="foe"]')?.textContent ?? "";
    const foeSide = () => preview()?.querySelector("[data-foe]")?.getAttribute("data-foe");
    const card = (slot: number) => preview()?.querySelector<HTMLElement>(`[data-preview-slot="${slot}"]`);
    const segment = (label: string) => [...(preview()?.querySelectorAll<HTMLButtonElement>("[data-foe] button") ?? [])].find((b) => b.textContent === label);
    // Step „Trainer“: the class picture, the trainer in the battle box, the empty first place.
    expect(preview()).not.toBeNull();
    expect(getTrainerSprite).toHaveBeenCalledWith(0);
    expect(preview()?.querySelector("[data-trainer-sprite] img")).not.toBeNull();
    expect(foe()).toContain("Pokémon-Trainer");
    expect(card(0)?.getAttribute("data-state")).toBe("empty");
    await setValue(container.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jens");
    expect(foe()).toContain("Pokémon-Trainer Jens");
    // A card of the preview picks the species of its place, from any step.
    await click(card(0));
    await flush();
    await click(options().find((o) => o.textContent?.includes("Glumanda")));
    await flush();
    expect(card(0)?.getAttribute("data-species")).toBe("4");
    expect(card(0)?.textContent).toContain("Glumanda");
    await next();
    expect(current()).toBe("2. Team");
    expect(preview()).not.toBeNull();
    // The rows of the team carry the menu icon, like the cards of the preview.
    expect(getMonIcon).toHaveBeenCalledWith(4);
    expect(rows()[0]?.querySelector("[data-species] img[data-sprite=picture]")).not.toBeNull();
    expect(card(0)?.querySelector("img[data-sprite=picture]")).not.toBeNull();
    await pickSpecies(rows()[0]!, "Bisasam");
    expect(card(0)?.getAttribute("data-species")).toBe("1");
    await click(segment("Erstes Pokémon"));
    await flush();
    expect(foeSide()).toBe("pokemon");
    expect(foe()).toContain("Bisasam");
    expect(getBattleSprite).toHaveBeenLastCalledWith({ species: 1, side: "front" });
    await setValue(rows()[0]!.querySelector<HTMLInputElement>('input[aria-label="Level"]')!, "12");
    expect(foe()).toContain("Lv. 12");
    await click(button("Pokémon hinzufügen"));
    expect(card(1)?.getAttribute("data-state")).toBe("empty");
    await click(button("Pokémon 2 entfernen"));
    expect(card(1)).toBeNull();
    // The preview stands beside every step and keeps the chosen side.
    await next();
    expect(current()).toBe("3. Texte");
    expect(foeSide()).toBe("pokemon");
    await next();
    expect(current()).toBe("4. Wo");
    expect(foeSide()).toBe("pokemon");
    await next();
    expect(current()).toBe("5. Zusammenfassung");
    expect(foe()).toContain("Bisasam");
    expect(container.querySelectorAll("[data-trainer-preview]")).toHaveLength(1);
  });
});
