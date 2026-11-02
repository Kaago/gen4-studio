// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { entityIds, freeFlag, listFlags, storyCreate, storyGet, storyUpdate, threadGraph } from "../api";
import type { FlagListDto, ProjectInfo } from "../api";
import { FLAGS, flagEntry, flagList } from "../flags/flagTesting";
import { invalidateFlagLists } from "../flags/useFlagList";
import { typeInto } from "../ui/primitives/testUtils";
import { graphOf, storiesOf, storyDto, storyReport, storyRow } from "./storiesTesting";
import { StoriesSurface } from "./StoriesSurface";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  storyGet: vi.fn(),
  storyCreate: vi.fn(),
  storyUpdate: vi.fn(),
  threadGraph: vi.fn(),
  listFlags: vi.fn(),
  freeFlag: vi.fn(),
  entityIds: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 1 };
const VARS: FlagListDto = flagList("var", [flagEntry(0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN" }), flagEntry(0x406c, { name: "VAR_UNUSED_0x406C", free: true })]);
const STORIES = storiesOf([
  storyRow(0, { name: "Rivalin", progress: { var: "VAR_RIVALIN" }, steps: 3 }),
  storyRow(1, { name: "Medaille", progress: { flag: "FLAG_HAT_MEDAILLE" }, steps: 0 }),
]);

type Props = ComponentProps<typeof StoriesSurface>;

describe("StoriesSurface", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  const goTo = vi.fn();
  const onOpenTextProject = vi.fn();
  const onCreated = vi.fn();
  const onUpdated = vi.fn();
  const onError = vi.fn();
  beforeAll(async () => {
    await import("./graph/ThreadGraph");
  }, 60_000);
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    invalidateFlagLists();
    vi.mocked(storyGet).mockReset().mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    vi.mocked(storyCreate).mockReset().mockResolvedValue(storyReport(2));
    vi.mocked(storyUpdate).mockReset().mockResolvedValue(storyReport(0, { files: ["data/stories/0000.json"] }));
    vi.mocked(threadGraph).mockReset().mockImplementation(async (variable) => graphOf(variable));
    vi.mocked(listFlags).mockReset().mockImplementation(async (kind) => (kind === "flag" ? FLAGS : VARS));
    vi.mocked(freeFlag).mockReset().mockResolvedValue({ kind: "var", number: 0x406c, name: "VAR_UNUSED_0x406C", pool_free: 1, scripts_indexed: true });
    vi.mocked(entityIds).mockReset().mockResolvedValue({});
    for (const fn of [onSelect, goTo, onOpenTextProject, onCreated, onUpdated, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (over: Partial<Props> = {}) => {
    await act(async () =>
      root.render(
        <StoriesSurface
          stories={STORIES}
          error={null}
          selected={null}
          onSelect={onSelect}
          textProject
          onOpenTextProject={onOpenTextProject}
          depth="bauen"
          goTo={goTo}
          project={project}
          revision={0}
          onCreated={onCreated}
          onUpdated={onUpdated}
          onError={onError}
          {...over}
        />,
      ),
    );
    await flush();
  };
  const surface = () => container.querySelector<HTMLElement>("[data-geschichten-surface=stories]")!;
  const table = () => surface().querySelector<HTMLElement>('[role=grid][aria-label="Geschichten"]');
  const row = (id: string) => surface().querySelector<HTMLElement>(`[data-story-row="${id}"]`)!;
  const cells = (id: string) => [...row(id).querySelectorAll("[role=gridcell]")].map((c) => c.textContent);
  const newButton = () => surface().querySelector<HTMLButtonElement>("[data-new-story]")!;
  const dialog = () => document.querySelector<HTMLElement>("[data-new-story-dialog]");
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());
  const reason = (el: HTMLElement | undefined) => document.getElementById(el?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("rows_show_name_progress_and_step_count", async () => {
    await show();
    expect(surface().querySelector("h1")?.textContent).toBe("Geschichten");
    expect(newButton().textContent).toBe("Neue Geschichte…");
    expect([...table()!.querySelectorAll("[role=columnheader]")].map((h) => h.textContent)).toEqual(["Name", "Fortschritt", "Schritte"]);
    expect(table()!.querySelectorAll("[data-story-row]")).toHaveLength(2);
    // The progress as a chip of the area Geschichten with its number (from Bauen), the steps as a count.
    expect(cells("STORY_0000")).toEqual(["Rivalin", "VAR_RIVALIN0x4040", "3"]);
    expect(cells("STORY_0001")).toEqual(["Medaille", "FLAG_HAT_MEDAILLE0x0094", "0"]);
    expect(surface().textContent).toContain("2 Geschichten");
    // The chip opens Geschichten › Flags und Variablen on the number.
    await click(row("STORY_0001").querySelector("[data-story-progress]"));
    expect(goTo).toHaveBeenCalledWith({ kind: "flag", flagKind: "flag", number: 0x94 });
    // In Erzählen the number is hidden.
    await show({ depth: "erzaehlen" });
    expect(cells("STORY_0000")[1]).toBe("VAR_RIVALIN");
    // A row chosen goes to the app; the chosen one shows its detail below.
    await click(row("STORY_0000"));
    expect(onSelect).toHaveBeenCalledWith(0);
    expect(surface().querySelector("[data-story-detail]")).toBeNull();
    await show({ selected: 0 });
    expect(row("STORY_0000").getAttribute("aria-selected")).toBe("true");
    expect(storyGet).toHaveBeenLastCalledWith("STORY_0000");
    expect(surface().querySelector("[data-story-detail]")?.getAttribute("data-story-detail")).toBe("STORY_0000");
    // The detail's writes go through story_update and are reported.
    typeInto(surface().querySelector<HTMLTextAreaElement>("[data-story-notes] textarea")!, "Neu.");
    await act(async () => void surface().querySelector("[data-story-notes] textarea")!.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await flush();
    expect(storyUpdate).toHaveBeenCalledWith("STORY_0000", { notes: "Neu." });
    expect(onUpdated).toHaveBeenCalledWith(storyReport(0, { files: ["data/stories/0000.json"] }));
    // No stories yet: the way to the first.
    await show({ stories: storiesOf([]) });
    expect(table()).toBeNull();
    expect(surface().textContent).toContain("Noch keine Geschichten.");
    // While the list is read, and when it cannot be.
    await show({ stories: null });
    expect(surface().textContent).toContain("Wird gelesen …");
    await show({ stories: null, error: "Die Geschichten sind nicht lesbar." });
    expect(surface().querySelector("[role=alert]")?.textContent).toBe("Die Geschichten sind nicht lesbar.");
  });

  it("creating_reports_and_selects_the_new_story", async () => {
    await show();
    expect(newButton().getAttribute("aria-disabled")).toBeNull();
    await click(newButton());
    expect(dialog()).not.toBeNull();
    typeInto(dialog()!.querySelector<HTMLInputElement>('[aria-label="Name"]')!, "Rivalin am See");
    await click(dialog()!.querySelector('[aria-label="Fortschritt wählen…"]'));
    await flush();
    await click([...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Freie Variable nehmen"));
    await flush();
    await click([...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Geschichte anlegen"));
    await flush();
    expect(storyCreate).toHaveBeenCalledWith({ name: "Rivalin am See", progress: { var: "VAR_UNUSED_0x406C" } });
    // The app hears the report with the name (its toast) and the new story is chosen; the dialog has closed.
    expect(onCreated).toHaveBeenCalledWith(storyReport(2), "Rivalin am See");
    expect(onSelect).toHaveBeenCalledWith(2);
    expect(dialog()).toBeNull();
    // What failed after writing goes to the app's errors.
    vi.mocked(storyCreate).mockResolvedValueOnce(storyReport(3, { warnings: ["Der Verlauf ist nicht lesbar."] }));
    await click(newButton());
    typeInto(dialog()!.querySelector<HTMLInputElement>('[aria-label="Name"]')!, "Zweite");
    await click(dialog()!.querySelector('[aria-label="Fortschritt wählen…"]'));
    await flush();
    await click([...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent?.includes("0x4040")));
    await flush();
    await click([...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Geschichte anlegen"));
    await flush();
    expect(onError).toHaveBeenCalledWith("Der Verlauf ist nicht lesbar.");
  });

  it("the_dspre_layout_locks_creating_with_the_way_out", async () => {
    await show({ textProject: false, stories: storiesOf([], { text_project: false }) });
    // Locked but focusable, with the kernel's reason as its description and tooltip; a click opens nothing.
    expect(newButton().getAttribute("aria-disabled")).toBe("true");
    expect(reason(newButton())).toBe("Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.");
    await click(newButton());
    expect(dialog()).toBeNull();
    expect(storyCreate).not.toHaveBeenCalled();
    // Below it the way out: Technik › Textprojekt.
    const setUp = [...surface().querySelectorAll("button")].find((b) => b.textContent === "Als Textprojekt einrichten…")!;
    await click(setUp);
    expect(onOpenTextProject).toHaveBeenCalledOnce();
    expect(table()).toBeNull();
  });

  it("a_row_with_a_problem_shows_the_notice", async () => {
    await show({ stories: storiesOf([storyRow(0, { name: "Rivalin" }), storyRow(1, { name: "", progress: null, steps: 0, problem: "data/stories/0001.json:3: unbekanntes Feld „note“." })]) });
    const broken = row("STORY_0001");
    // The problem as a warning in the row, no progress chip and no count.
    const notice = broken.querySelector<HTMLElement>("[role=status]")!;
    expect(notice.textContent).toContain("data/stories/0001.json:3: unbekanntes Feld „note“.");
    expect(broken.querySelector("[data-story-progress]")).toBeNull();
    expect(cells("STORY_0001")[2]).toBe("–");
    // The readable row beside it as ever.
    expect(row("STORY_0000").querySelector("[data-story-progress]")).not.toBeNull();
  });
});
