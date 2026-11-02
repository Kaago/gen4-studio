// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { entityIds, flagUsage, listFlags, listThreads, storyGet, storyList, threadGraph } from "../api";
import type { ProjectInfo } from "../api";
import { FLAGS, flagUsageOf } from "../flags/flagTesting";
import { invalidateFlagLists } from "../flags/useFlagList";
import { GeschichtenArea } from "./GeschichtenArea";
import type { GeschichtenAreaProps, GeschichtenEntry } from "./GeschichtenArea";
import { THREADS, graphOf, storiesOf, storyDto, storyRow } from "./storiesTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  listThreads: vi.fn(),
  threadGraph: vi.fn(),
  storyList: vi.fn(),
  storyGet: vi.fn(),
  entityIds: vi.fn(),
  listFlags: vi.fn(),
  flagUsage: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 };

describe("GeschichtenArea", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelectFlag = vi.fn();
  const onOpenTextProject = vi.fn();
  const onOpenProject = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    invalidateFlagLists();
    vi.mocked(listThreads).mockReset().mockResolvedValue(THREADS);
    vi.mocked(threadGraph).mockReset().mockImplementation(async (variable) => graphOf(variable));
    vi.mocked(storyList).mockReset().mockResolvedValue(storiesOf([storyRow(0), storyRow(1)]));
    vi.mocked(storyGet).mockReset().mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    vi.mocked(entityIds).mockReset().mockResolvedValue({});
    vi.mocked(listFlags).mockReset().mockResolvedValue(FLAGS);
    vi.mocked(flagUsage).mockReset().mockImplementation(async (kind, number) => flagUsageOf(kind, number));
    onSelectFlag.mockReset();
    onOpenTextProject.mockReset();
    onOpenProject.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** The area with the entry and the selections held as the app holds them. */
  function Host({ start = "threads", withProject = true, ...over }: Partial<GeschichtenAreaProps> & { start?: GeschichtenEntry; withProject?: boolean }) {
    const [entry, setEntry] = useState<GeschichtenEntry>(start);
    const [thread, setThread] = useState<number | null>(null);
    const [story, setStory] = useState<number | null>(null);
    return (
      <GeschichtenArea
        project={withProject ? info.root : null}
        listOpen
        onToggleList={() => {}}
        depth="bauen"
        revision={0}
        goTo={() => {}}
        selectedThread={thread}
        onSelectThread={setThread}
        selectedStory={story}
        onSelectStory={setStory}
        flags={withProject ? { project: info, revision: 0, selected: null, onSelect: onSelectFlag, readOnly: false, onError: () => {} } : undefined}
        textProject
        onOpenTextProject={onOpenTextProject}
        onOpenProject={onOpenProject}
        onError={() => {}}
        {...over}
        entry={entry}
        onEntry={setEntry}
      />
    );
  }
  const render = (props: Partial<GeschichtenAreaProps> & { start?: GeschichtenEntry; withProject?: boolean } = {}) => act(async () => root.render(<Host {...props} />));
  const settle = () => act(async () => {});
  const rows = () => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Geschichten"] [role=option]')];
  const label = (r: HTMLElement) => document.getElementById(r.getAttribute("aria-labelledby") ?? "")?.textContent;
  const row = (text: string) => rows().find((r) => label(r) === text);
  const subtitle = (text: string) => document.getElementById(row(text)?.getAttribute("aria-describedby") ?? "")?.textContent;
  const area = () => container.querySelector<HTMLElement>("[data-geschichten]");
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("three_entries_and_their_surfaces", async () => {
    // Without a project the entries say so and nothing is read.
    await render({ withProject: false });
    expect(rows().map(label)).toEqual(["Fäden", "Geschichten", "Flags und Variablen"]);
    expect(subtitle("Fäden")).toBe("Kein Projekt");
    expect(subtitle("Geschichten")).toBe("Kein Projekt");
    expect(subtitle("Flags und Variablen")).toBe("Kein Projekt");
    expect(listThreads).not.toHaveBeenCalled();
    expect(storyList).not.toHaveBeenCalled();
    // Each surface says so and offers „Projekt öffnen“ (Fäden, Geschichten, and Flags und Variablen as in Technik before).
    const openButton = () => [...container.querySelectorAll("button")].find((b) => b.textContent === "Projekt öffnen");
    for (const entry of ["Fäden", "Geschichten", "Flags und Variablen"]) {
      await click(row(entry));
      expect(area()?.textContent).toContain("Kein Projekt geöffnet.");
      expect(area()?.textContent).not.toContain("Wird gelesen …");
      await click(openButton());
    }
    expect(onOpenProject).toHaveBeenCalledTimes(3);
    await click(row("Fäden"));
    // With one: each entry with its symbol and its count.
    await render();
    await settle();
    for (const r of rows()) expect(r.querySelector("svg")).not.toBeNull();
    expect(subtitle("Fäden")).toBe("3 Fäden");
    expect(subtitle("Geschichten")).toBe("2 Geschichten");
    expect(subtitle("Flags und Variablen")).toBe("5 Flags · 3 frei");
    // Fäden first: the table of threads, one read for the list and the surface.
    expect(area()?.getAttribute("data-geschichten")).toBe("threads");
    expect(row("Fäden")?.getAttribute("aria-selected")).toBe("true");
    expect(container.querySelectorAll("[data-thread-row]")).toHaveLength(3);
    expect(listThreads).toHaveBeenCalledOnce();
    // A thread chosen is held outside and shows its graph.
    await click(container.querySelector("[data-thread-row='16448']"));
    await settle();
    expect(container.querySelector("[data-thread-row='16448']")?.getAttribute("aria-selected")).toBe("true");
    expect(threadGraph).toHaveBeenLastCalledWith(0x4040, "STORY_RIVALIN");
    // Geschichten: the surface of the stories.
    await click(row("Geschichten"));
    expect(area()?.getAttribute("data-geschichten")).toBe("stories");
    expect(container.querySelector("[data-geschichten-surface=stories] h1")?.textContent).toBe("Geschichten");
    // Counts in the singular, and none.
    vi.mocked(listThreads).mockResolvedValue({ ...THREADS, threads: THREADS.threads.slice(0, 1) });
    vi.mocked(storyList).mockResolvedValue(storiesOf([storyRow(0)]));
    await render({ revision: 1 });
    await settle();
    expect(subtitle("Fäden")).toBe("1 Faden");
    expect(subtitle("Geschichten")).toBe("1 Geschichte");
    vi.mocked(listThreads).mockResolvedValue({ ...THREADS, threads: [] });
    vi.mocked(storyList).mockResolvedValue(storiesOf([]));
    await render({ revision: 2 });
    await settle();
    expect(subtitle("Fäden")).toBe("Keine Fäden");
    expect(subtitle("Geschichten")).toBe("Keine Geschichten");
    // A list that cannot be read says so.
    vi.mocked(listThreads).mockRejectedValue({ code: "refused", message: "Der Index ist nicht lesbar.", asset: null });
    await render({ revision: 3 });
    await settle();
    expect(subtitle("Fäden")).toBe("Nicht lesbar");
  });

  it("the_flags_entry_mounts_the_flags_surface", async () => {
    await render({ start: "flags" });
    await settle();
    expect(area()?.getAttribute("data-geschichten")).toBe("flags");
    const surface = container.querySelector<HTMLElement>("[data-geschichten=flags] [data-technik=flags]")!;
    expect(surface.querySelector("h1")?.textContent).toBe("Flags und Variablen");
    expect(surface.querySelectorAll("[data-flag-row]")).toHaveLength(5);
    // The list and the surface share one read of the flags.
    expect(listFlags).toHaveBeenCalledOnce();
    // A row chosen goes to the app, which holds the selection.
    await click(surface.querySelector("[data-flag-row='148']"));
    expect(onSelectFlag).toHaveBeenCalledWith({ kind: "flag", number: 0x94 });
  });

  it("the_stories_entry_mounts_the_surface", async () => {
    await render({ start: "stories" });
    await settle();
    expect(area()?.getAttribute("data-geschichten")).toBe("stories");
    const surface = () => container.querySelector<HTMLElement>("[data-geschichten=stories] [data-geschichten-surface=stories]")!;
    expect(surface().querySelector("h1")?.textContent).toBe("Geschichten");
    expect(surface().querySelectorAll("[data-story-row]")).toHaveLength(2);
    expect(surface().querySelector("[data-new-story]")?.getAttribute("aria-disabled")).toBeNull();
    // The list and the surface share one read of the stories.
    expect(storyList).toHaveBeenCalledOnce();
    // A row chosen is held outside (as a link would set it) and shows its detail.
    await click(surface().querySelector("[data-story-row=STORY_0001]"));
    await settle();
    expect(surface().querySelector("[data-story-row=STORY_0001]")?.getAttribute("aria-selected")).toBe("true");
    expect(storyGet).toHaveBeenLastCalledWith("STORY_0001");
    expect(surface().querySelector("[data-story-detail]")?.getAttribute("data-story-detail")).toBe("STORY_0001");
    // In the DSPRE layout creating is locked with the way out.
    await render({ start: "stories", textProject: false });
    await settle();
    expect(surface().querySelector("[data-new-story]")?.getAttribute("aria-disabled")).toBe("true");
  });
});
