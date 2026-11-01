// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { entityIds, listFlags, threadGraph } from "../api";
import type { FlagListDto, ProjectInfo } from "../api";
import { FLAGS, flagEntry, flagList } from "../flags/flagTesting";
import { invalidateFlagLists } from "../flags/useFlagList";
import { typeInto } from "../ui/primitives/testUtils";
import { graphOf, storyDto } from "./storiesTesting";
import { StoryDetail } from "./StoryDetail";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  threadGraph: vi.fn(),
  listFlags: vi.fn(),
  entityIds: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 1 };
const VARS: FlagListDto = flagList("var", [flagEntry(0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN" })]);

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

type Props = ComponentProps<typeof StoryDetail>;

describe("StoryDetail", () => {
  let container: HTMLElement;
  let root: Root;
  const goTo = vi.fn();
  const onUpdate = vi.fn<Props["onUpdate"]>();
  // The graph is a lazy chunk: loaded once here, its `React.lazy` resolves at once.
  beforeAll(async () => {
    await import("./graph/ThreadGraph");
  }, 60_000);
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    invalidateFlagLists();
    vi.mocked(threadGraph).mockReset().mockImplementation(async (variable) => graphOf(variable));
    vi.mocked(listFlags).mockReset().mockImplementation(async (kind) => (kind === "flag" ? FLAGS : VARS));
    vi.mocked(entityIds).mockReset().mockImplementation(async (kind): Promise<Record<number, string>> => (kind === "Trainer" ? { 42: "TRAINER_0042" } : kind === "Map" ? { 411: "MAP_ZWEIBLATTDORF" } : {}));
    goTo.mockReset();
    onUpdate.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (over: Partial<Props> = {}) => {
    await act(async () => root.render(<StoryDetail story={storyDto(0)} depth="bauen" goTo={goTo} onUpdate={onUpdate} project={project} revision={0} {...over} />));
    await flush();
  };
  const detail = () => container.querySelector<HTMLElement>("[data-story-detail]")!;
  const notes = () => detail().querySelector<HTMLTextAreaElement>("[data-story-notes] textarea")!;
  const nameField = () => detail().querySelector<HTMLInputElement>('input[aria-label="Name"]')!;
  const steps = () => detail().querySelector<HTMLElement>('[role=grid][aria-label="Schritte"]');
  const step = (key: string) => detail().querySelector<HTMLElement>(`[data-story-step="${key}"]`)!;
  const cells = (key: string) => [...step(key).querySelectorAll("[role=gridcell]")].map((c) => c.textContent);
  const blur = (el: HTMLElement) => act(async () => void el.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("notes_commit_on_blur", async () => {
    await show();
    expect(detail().getAttribute("data-story-detail")).toBe("STORY_0000");
    expect(notes().value).toBe("Die Rivalin begleitet den Weg.");
    // Typing writes nothing; leaving the field does, once, with the notes alone.
    const pending = deferred<void>();
    onUpdate.mockImplementationOnce(() => pending.promise);
    typeInto(notes(), "Die Rivalin wartet am See.");
    expect(onUpdate).not.toHaveBeenCalled();
    await blur(notes());
    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({ notes: "Die Rivalin wartet am See." });
    // The write takes seconds: the fields are locked, the typed text stays and the detail says it saves.
    expect(notes().value).toBe("Die Rivalin wartet am See.");
    expect(notes().disabled).toBe(true);
    expect(nameField().disabled).toBe(true);
    expect(detail().textContent).toContain("Wird gespeichert …");
    await act(async () => pending.resolve());
    await flush();
    expect(notes().disabled).toBe(false);
    expect(detail().textContent).not.toContain("Wird gespeichert …");
    // The story read back holds the new notes.
    await show({ story: storyDto(0, {}, { notes: "Die Rivalin wartet am See." }) });
    expect(notes().value).toBe("Die Rivalin wartet am See.");
    // The name commits the same way, trimmed; an empty name is not sent.
    typeInto(nameField(), "  Rivalin am See ");
    await blur(nameField());
    expect(onUpdate).toHaveBeenLastCalledWith({ name: "Rivalin am See" });
    onUpdate.mockClear();
    typeInto(nameField(), "  ");
    await blur(nameField());
    expect(onUpdate).not.toHaveBeenCalled();
    // The name sent last stays until the story is read back.
    expect(nameField().value).toBe("Rivalin am See");
    // A refusal stays visible under the fields; the field shows the story's text again.
    onUpdate.mockRejectedValueOnce({ code: "refused", message: "Die Datei data/stories/0000.json ist nicht schreibbar.", asset: null });
    typeInto(notes(), "Weg.");
    await blur(notes());
    await flush();
    expect(detail().querySelector("[role=alert]")?.textContent).toBe("Die Datei data/stories/0000.json ist nicht schreibbar.");
    expect(notes().value).toBe("Die Rivalin wartet am See.");
    expect(notes().disabled).toBe(false);
  });

  it("steps_show_value_kind_name_and_uses", async () => {
    await show();
    expect([...steps()!.querySelectorAll("[role=columnheader]")].map((h) => h.textContent)).toEqual(["Wert", "Art", "Name", "Verweise"]);
    expect(cells("s1").slice(0, 3)).toEqual(["1", "Dialog", "Rivalin trifft"]);
    expect(cells("s2").slice(0, 3)).toEqual(["2", "Kampf", "Kampf am See"]);
    expect(step("s2").querySelector("[data-story-kind]")?.getAttribute("data-story-kind")).toBe("battle");
    // The uses as chips that open where they lead; a use no link knows is text.
    const use = (key: string, n: number) => step(key).querySelectorAll<HTMLElement>("[data-story-use]")[n]!;
    expect(use("s1", 0).textContent).toContain("MAP_ZWEIBLATTDORF");
    await click(use("s1", 0).matches("button") ? use("s1", 0) : use("s1", 0).querySelector("button"));
    expect(goTo).toHaveBeenLastCalledWith({ kind: "header", id: 411 });
    const open = async (key: string, n: number) => click(use(key, n).matches("button") ? use(key, n) : use(key, n).querySelector("button"));
    expect(use("s1", 1).textContent).toContain("Textarchiv 0012 · Text 4");
    await open("s1", 1);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "textArchive", index: 12, message: 4 });
    expect(use("s2", 0).textContent).toContain("TRAINER_0042");
    await open("s2", 0);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "trainer", index: 42 });
    expect(use("s2", 1).textContent).toContain("Scriptdatei 0101 · Script 2");
    await open("s2", 1);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "script", file: 101 });
    // A flag by its own name, a variable by its fallback id.
    expect(use("s3", 0).textContent).toContain("FLAG_HAT_MEDAILLE");
    await open("s3", 0);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "flag", flagKind: "flag", number: 0x94 });
    await open("s3", 1);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "flag", flagKind: "var", number: 0x4040 });
    expect(use("s3", 2).textContent).toBe("Objekt 3 · Eventdatei 0394");
    expect(use("s3", 2).querySelector("button")).toBeNull();
    expect(use("s3", 3).textContent).toContain("Level-Script 0012 · Bedingung 1");
    await open("s3", 3);
    expect(goTo).toHaveBeenLastCalledWith({ kind: "script", file: 12 });
    // Steps are only read here: no field in the table.
    expect(steps()!.querySelector("input, textarea")).toBeNull();
    // Without steps: where they come from.
    await show({ story: storyDto(0, {}, { steps: undefined }) });
    expect(steps()).toBeNull();
    expect(detail().textContent).toContain("Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7).");
  });

  it("a_story_without_values_says_so", async () => {
    // A free variable no script uses: the graph has no node.
    vi.mocked(threadGraph).mockImplementation(async (variable) => graphOf(variable, { nodes: [], edges: [] }));
    await show({ story: storyDto(0, { progress_number: 0x406c }, { progress: { var: "VAR_UNUSED_0x406C" }, steps: undefined }) });
    expect(threadGraph).toHaveBeenCalledWith(0x406c, "STORY_0000");
    expect(detail().querySelector("[data-story-empty-graph]")?.textContent).toBe("Noch keine Werte im Spiel");
    expect(detail().querySelector("[data-thread-graph]")).toBeNull();
    // A progress whose id names nothing: nothing to ask.
    vi.mocked(threadGraph).mockClear();
    await show({ story: storyDto(0, { progress_number: null }, { progress: { var: "VAR_WEG" } }) });
    expect(threadGraph).not.toHaveBeenCalled();
    expect(detail().querySelector("[data-story-empty-graph]")?.textContent).toBe("Noch keine Werte im Spiel");
    // A flag: thread_graph takes a variable only (3d.6), the flag's number is never asked as one.
    await show({ story: storyDto(0, { progress_number: 0x94 }, { progress: { flag: "FLAG_HAT_MEDAILLE" } }) });
    expect(threadGraph).not.toHaveBeenCalled();
    expect(detail().querySelector("[data-story-empty-graph]")?.textContent).toBe("Noch keine Werte im Spiel");
  });

  it("the_graph_carries_the_steps_on_value_nodes", async () => {
    await show();
    // The graph of the progress variable, asked with the story: its steps lie on the value nodes.
    expect(threadGraph).toHaveBeenCalledWith(0x4040, "STORY_0000");
    for (let i = 0; i < 50 && detail().querySelector("[data-thread-graph]") === null; i++) await flush();
    const graph = detail().querySelector<HTMLElement>("[data-thread-graph]")!;
    expect(graph.getAttribute("data-node-count")).toBe(String(graphOf(0x4040).nodes.length));
    expect(graph.querySelector('[data-node-id="v:1"] [data-story-node], [data-node-id="v:1"][data-story-node]')?.getAttribute("data-kind")).toBe("dialog");
    expect(graph.querySelector('[data-node-id="v:2"]')?.getAttribute("data-kind")).toBe("battle");
    expect(graph.querySelector('[data-node-id="v:3"]')?.hasAttribute("data-noted-only")).toBe(true);
    expect(detail().querySelector("[data-story-empty-graph]")).toBeNull();
    // The head: the progress as a chip that opens Geschichten › Flags und Variablen on its number.
    const chip = detail().querySelector<HTMLElement>("[data-story-progress]")!;
    expect(chip.textContent).toContain("VAR_RIVALIN");
    expect(chip.textContent).toContain("0x4040");
    await click(chip.matches("button") ? chip : chip.querySelector("button"));
    expect(goTo).toHaveBeenLastCalledWith({ kind: "flag", flagKind: "var", number: 0x4040 });
  });
});
