// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { threadGraph } from "../api";
import type { ProjectInfo } from "../api";
import { THREADS, graphOf, storyRow, threadList } from "./storiesTesting";
import { ThreadsSurface } from "./ThreadsSurface";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), threadGraph: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 };

type Props = ComponentProps<typeof ThreadsSurface>;

describe("ThreadsSurface", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  const goTo = vi.fn();
  const onOpenTextProject = vi.fn();
  const onOpenProject = vi.fn();
  // The graph is a lazy chunk: loaded once here, its `React.lazy` resolves at once (a slow machine takes seconds for the first import).
  beforeAll(async () => {
    await import("./graph/ThreadGraph");
  }, 60_000);
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(threadGraph).mockReset().mockImplementation(async (variable) => graphOf(variable));
    onSelect.mockReset();
    goTo.mockReset();
    onOpenTextProject.mockReset();
    onOpenProject.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<Props> = {}) =>
    act(async () =>
      root.render(
        <ThreadsSurface
          threads={THREADS}
          error={null}
          depth="bauen"
          selected={null}
          onSelect={onSelect}
          goTo={goTo}
          project={project}
          revision={0}
          onOpenTextProject={onOpenTextProject}
          onOpenProject={onOpenProject}
          {...over}
        />,
      ),
    );
  const table = () => container.querySelector<HTMLElement>('[role=grid][aria-label="Fäden"]');
  const row = (variable: number) => container.querySelector<HTMLElement>(`[data-thread-row="${variable}"]`)!;
  const cells = (variable: number) => [...row(variable).querySelectorAll("[role=gridcell]")].map((c) => c.textContent);

  it("rows_show_name_values_places_and_story", async () => {
    await show({ stories: [storyRow(0, { id: "STORY_RIVALIN", name: "Rivalin" })] });
    expect(container.querySelector("[data-geschichten-surface=threads] h1")?.textContent).toBe("Fäden");
    expect([...table()!.querySelectorAll("[role=columnheader]")].map((h) => h.textContent)).toEqual(["Faden", "Werte", "Orte", "Geschichte"]);
    expect(table()!.querySelectorAll("[data-thread-row]")).toHaveLength(3);
    expect(cells(0x4031)).toEqual(["VAR_UNK_0x40310x4031", "3 Werte: 0, 1, 2", "3 Orte: Zweiblattdorf, Route 201, …", "–"]);
    expect(cells(0x4055)[1]).toBe("16 Werte: 0 … 15");
    expect(cells(0x4055)[2]).toBe("7 Orte: Herzhofen, Route 205, Flori + 4");
    // The own name first; the story as a chip of the area Geschichten that opens it.
    expect(row(0x4040).querySelector("[data-thread-name]")?.textContent).toBe("VAR_RIVALIN");
    const chip = row(0x4040).querySelector<HTMLButtonElement>("[data-thread-story]")!;
    // The name alone in the narrow column (gate 3d.6, U4: „He…“ beside a whole STORY_HEIMKEHR); from Bauen the id is the chip's title.
    expect(chip.textContent).toBe("Rivalin");
    expect(chip.title).toBe("STORY_RIVALIN");
    await act(async () => chip.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "story", number: 0 });
    await show({ stories: [storyRow(0, { id: "STORY_RIVALIN", name: "Rivalin" })], depth: "erzaehlen" });
    expect(row(0x4040).querySelector<HTMLButtonElement>("[data-thread-story]")!.textContent).toBe("Rivalin");
    expect(row(0x4040).querySelector<HTMLButtonElement>("[data-thread-story]")!.hasAttribute("title")).toBe(false);
    await show({ stories: [storyRow(0, { id: "STORY_RIVALIN", name: "Rivalin" })] });
    // The count below the table.
    expect(container.textContent).toContain("3 Fäden");
  });

  it("in_erzaehlen_the_number_is_hidden_and_from_bauen_shown", async () => {
    await show({ depth: "erzaehlen" });
    expect(row(0x4040).querySelector("[data-thread-tech]")).toBeNull();
    expect(row(0x4040).textContent).not.toContain("0x4040");
    // An unnamed variable keeps its kind and number as its name.
    await show({ depth: "erzaehlen", threads: threadList([{ ...THREADS.threads[0]!, var: 0x4030, name: null }]) });
    expect(row(0x4030).querySelector("[data-thread-name]")?.textContent).toBe("Variable 0x4030");
    await show({ depth: "bauen" });
    expect(row(0x4031).querySelector("[data-thread-tech]")?.textContent).toBe("0x4031");
    await show({ depth: "tuefteln" });
    expect(row(0x4031).querySelector("[data-thread-tech]")?.textContent).toBe("0x4031");
  });

  it("selecting_a_row_loads_its_graph", async () => {
    await show();
    expect(container.querySelector("[data-thread-graph-box]")).toBeNull();
    expect(container.textContent).toContain("Wähle einen Faden");
    await act(async () => row(0x4040).click());
    expect(onSelect).toHaveBeenCalledWith(0x4040);
    await show({ selected: 0x4040 });
    expect(row(0x4040).getAttribute("aria-selected")).toBe("true");
    // The graph of the chosen thread, asked with its story; the lazy graph loads meanwhile.
    expect(threadGraph).toHaveBeenLastCalledWith(0x4040, "STORY_RIVALIN");
    for (let i = 0; i < 50 && container.querySelector("[data-thread-graph]") === null; i++) await flush();
    const graph = container.querySelector<HTMLElement>("[data-thread-graph]")!;
    expect(graph.getAttribute("data-node-count")).toBe(String(graphOf(0x4040).nodes.length));
    expect(container.querySelector("[data-thread-graph-box] h2")?.textContent).toBe("Graph: VAR_RIVALIN");
    // A place opens where its reference is.
    await act(async () => container.querySelector<HTMLElement>('[data-story-node][data-node-id="p:script:0101"]')!.click());
    await act(async () => container.querySelector<HTMLButtonElement>("[data-open-place] button")!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "script", file: 101, line: 12 });
    // A refusal shows as an error, not as a graph.
    vi.mocked(threadGraph).mockRejectedValueOnce({ code: "refused", message: "Der Index ist nicht lesbar.", asset: null });
    await show({ selected: 0x4031 });
    await flush();
    expect(container.querySelector("[data-thread-graph-box]")?.textContent).toContain("Der Index ist nicht lesbar.");
  });

  it("without_a_project_the_empty_state_offers_projekt_oeffnen", async () => {
    // No project: neither „Wird gelesen …“ forever nor a table, but the way out as the sibling surfaces offer it.
    await show({ project: null, threads: null });
    expect(table()).toBeNull();
    expect(container.textContent).not.toContain("Wird gelesen …");
    expect(container.textContent).toContain("Kein Projekt geöffnet.");
    const button = [...container.querySelectorAll("button")].find((b) => b.textContent === "Projekt öffnen")!;
    await act(async () => button.click());
    expect(onOpenProject).toHaveBeenCalledOnce();
    expect(threadGraph).not.toHaveBeenCalled();
  });

  it("without_adopted_scripts_the_empty_state_offers_the_way", async () => {
    await show({ threads: threadList([], { scripts_adopted: false }) });
    expect(table()).toBeNull();
    expect(container.textContent).toContain("Keine Fäden: die Scripts sind nicht übernommen.");
    const button = [...container.querySelectorAll("button")].find((b) => b.textContent === "Scripts übernehmen…")!;
    await act(async () => button.click());
    expect(onOpenTextProject).toHaveBeenCalledOnce();
    // Adopted, but no variable is a thread: no way to offer.
    await show({ threads: threadList([]) });
    expect(container.textContent).toContain("Keine Fäden");
    expect([...container.querySelectorAll("button")].some((b) => b.textContent === "Scripts übernehmen…")).toBe(false);
    // While the list is read, and when it cannot be.
    await show({ threads: null });
    expect(container.textContent).toContain("Wird gelesen …");
    await show({ threads: null, error: "Der Index ist nicht lesbar." });
    expect(container.textContent).toContain("Der Index ist nicht lesbar.");
  });
});
