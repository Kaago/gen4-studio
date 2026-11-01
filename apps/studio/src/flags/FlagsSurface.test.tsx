// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flagUsage, listFlags, nameFlag, planRenameId } from "../api";
import type { FlagKind, FlagListDto, NameReport, ProjectInfo } from "../api";
import { EntityIdCache, TextProjectProvider } from "../textproject/TextProjectContext";
import { typeInto } from "../ui/primitives/testUtils";
import { FLAGS, flagEntry, flagList, flagUsageOf } from "./flagTesting";
import { FlagsSurface, flagsSubtitle } from "./FlagsSurface";
import type { FlagsSurfaceProps } from "./FlagsSurface";
import { invalidateFlagLists } from "./useFlagList";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  listFlags: vi.fn(),
  flagUsage: vi.fn(),
  nameFlag: vi.fn(),
  planRenameId: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });
const VARS: FlagListDto = flagList("var", [flagEntry(0x4000, { name: "VAR_MAP_LOCAL_0x00", aliases: ["VARS_START"], area: "map_local" }), flagEntry(0x4030, { free: true })]);
type Selection = { kind: FlagKind; number: number } | null;

describe("FlagsSurface", () => {
  let container: HTMLElement;
  let root: Root;
  let flags: FlagListDto;
  const onError = vi.fn();
  const goTo = vi.fn();
  const onNamed = vi.fn();
  let p: ProjectInfo;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    flags = flagList("flag", FLAGS.entries, { text_project: true });
    vi.mocked(listFlags).mockReset().mockImplementation(async (kind) => (kind === "flag" ? flags : VARS));
    vi.mocked(flagUsage).mockReset().mockImplementation(async (kind, number) => {
      const entry = (kind === "flag" ? flags : VARS).entries.find((e) => e.number === number);
      return flagUsageOf(kind, number, { name: entry?.name ?? null, own_id: entry?.own_id ?? null, description: entry?.description ?? null, area: entry?.area ?? "general" });
    });
    vi.mocked(nameFlag).mockReset();
    vi.mocked(planRenameId).mockReset();
    for (const fn of [onError, goTo, onNamed]) fn.mockReset();
    invalidateFlagLists();
    p = project();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** The surface with its selection held as App holds it; `selected` from outside replaces it (a link). */
  function Host({ selected: from = null, ...over }: Partial<FlagsSurfaceProps> & { selected?: Selection }) {
    const [selected, setSelected] = useState<Selection>(from);
    const [given, setGiven] = useState(from);
    if (given !== from) {
      setGiven(from);
      setSelected(from);
    }
    return (
      <TextProjectProvider active ids={new EntityIdCache()} onRenamed={() => {}}>
        <FlagsSurface project={p} revision={0} selected={selected} onSelect={setSelected} goTo={goTo} depth="bauen" readOnly={false} onError={onError} onNamed={onNamed} {...over} />
      </TextProjectProvider>
    );
  }
  const show = async (over: Partial<FlagsSurfaceProps> & { selected?: Selection } = {}) => {
    await act(async () => root.render(<Host {...over} />));
    await flush();
  };
  const surface = () => container.querySelector<HTMLElement>("[data-technik=flags]")!;
  const filterRow = () => surface().querySelector<HTMLElement>("[data-flag-filter]")!;
  const chip = (group: string, label: string) =>
    [...filterRow().querySelectorAll<HTMLButtonElement>(`[role=group][aria-label="${group}"] button`)].find((b) => b.textContent === label)!;
  const box = (label: string) => [...filterRow().querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === label)!.querySelector<HTMLElement>("[role=checkbox]")!;
  const search = () => filterRow().querySelector<HTMLInputElement>('input[aria-label="Name oder Nummer suchen"]')!;
  const rows = () => [...surface().querySelectorAll<HTMLElement>("[data-flag-row]")].map((r) => Number(r.getAttribute("data-flag-row")));
  const row = (n: number) => surface().querySelector<HTMLElement>(`[data-flag-row="${n}"]`)!;
  const detail = () => surface().querySelector<HTMLElement>("[data-flag-detail]")!;
  const button = (name: string, scope: ParentNode = surface()) => [...scope.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  const click = async (el: HTMLElement | undefined) => {
    await act(async () => el!.click());
    await flush();
  };

  it("filter_list_and_detail_work_together", async () => {
    await show();
    expect(surface().querySelector("h1")?.textContent).toBe("Flags und Variablen");
    expect(surface().textContent).toContain(
      "Welche Flags und Variablen das Spiel kennt und wo Scripts, Figuren, Auslöser, Level-Scripts und Trainer sie setzen oder prüfen.",
    );
    expect(listFlags).toHaveBeenCalledWith("flag");
    expect(rows()).toEqual([0x20, 0x94, 0x96, 0x173, 0x550]);
    expect(detail().textContent).toBe("Wähle links ein Flag oder eine Variable.");
    // The kind and the area: one chip each pressed; „Alle“ and the areas of flags (hgss' temporary ones only where the list has them).
    expect(chip("Art", "Flags").getAttribute("aria-pressed")).toBe("true");
    expect([...filterRow().querySelectorAll('[role=group][aria-label="Bereich"] button')].map((b) => b.textContent)).toEqual([
      "Alle",
      "kartenlokal",
      "allgemein",
      "versteckte Items",
      "Trainer",
      "System",
      "Tagesflags",
    ]);
    expect(filterRow().textContent).not.toContain("Filter lösen");
    // The search by an own name, then the boxes, then an area.
    typeInto(search(), "medaille");
    await flush();
    expect(rows()).toEqual([0x94]);
    await click(box("nur frei"));
    expect(rows()).toEqual([]);
    expect(surface().textContent).toContain("Nichts passt zu Filter und Suche.");
    await click(box("nur frei"));
    typeInto(search(), "");
    await flush();
    await click(box("nur mit eigenem Namen"));
    expect(rows()).toEqual([0x94]);
    await click(box("nur mit eigenem Namen"));
    await click(box("nur unbenutzt"));
    expect(rows()).toEqual([0x173]);
    await click(box("nur unbenutzt"));
    await click(chip("Bereich", "Trainer"));
    expect(chip("Bereich", "Trainer").getAttribute("aria-pressed")).toBe("true");
    expect(chip("Bereich", "Alle").getAttribute("aria-pressed")).toBe("false");
    expect(rows()).toEqual([0x550]);
    // Pressing the chosen chip again changes nothing; „Alle“ shows all again.
    await click(chip("Bereich", "Trainer"));
    expect(rows()).toEqual([0x550]);
    await click(chip("Bereich", "Alle"));
    expect(rows()).toHaveLength(5);
    // A row shows its usage in the detail beside the list.
    await click(row(0x94));
    expect(row(0x94).getAttribute("aria-selected")).toBe("true");
    expect(flagUsage).toHaveBeenLastCalledWith("flag", 0x94);
    expect(detail().querySelector("h2")?.textContent).toBe("FLAG_HAT_MEDAILLE");
    expect(surface().querySelector("[data-flag-split] [data-flag-list] + [data-flag-detail]")).not.toBeNull();
    // The variables: their list, their areas, nothing chosen any more.
    await click(chip("Art", "Variablen"));
    expect(listFlags).toHaveBeenCalledWith("var");
    expect(rows()).toEqual([0x4000, 0x4030]);
    expect([...filterRow().querySelectorAll('[role=group][aria-label="Bereich"] button')].map((b) => b.textContent)).toEqual([
      "Alle",
      "kartenlokal",
      "Objektgrafik",
      "allgemein",
      "Script-lokal",
    ]);
    expect(detail().textContent).toBe("Wähle links ein Flag oder eine Variable.");
    typeInto(search(), "VARS_START");
    await flush();
    expect(rows()).toEqual([0x4000]);
  });

  it("names_a_number_and_reads_the_list_again", async () => {
    await show({ selected: { kind: "flag", number: 0x96 } });
    // A text project with indexed scripts: no notice above the list.
    expect(surface().querySelector("[role=status]")).toBeNull();
    const reads = vi.mocked(listFlags).mock.calls.length;
    const named: NameReport = { kind: "flag", number: 0x96, id: "FLAG_HAT_ORDEN", created: true, files: [], change: 3, ms: 2, warnings: [] };
    vi.mocked(nameFlag).mockResolvedValueOnce(named);
    await click(button("Als eigenes Flag benennen…"));
    const dialog = document.querySelector<HTMLElement>("[data-name-flag-dialog]")!;
    expect(dialog.querySelector("h2")?.textContent).toBe("Als eigenes Flag benennen");
    typeInto(dialog.querySelector<HTMLInputElement>('[aria-label="Eigener Name"]')!, "FLAG_HAT_ORDEN");
    await click(button("Benennen", dialog));
    // The report goes to the app with its sentence; the list and the usage are read again (the chips of the inspector too).
    expect(onNamed).toHaveBeenCalledWith(named, "FLAG_HAT_ORDEN benannt (ein Schritt im Verlauf).");
    expect(vi.mocked(listFlags).mock.calls.length).toBeGreaterThan(reads);
    expect(document.querySelector("[data-name-flag-dialog]")).toBeNull();
    // „Umbenennen…“ of an own name opens the text project's rename of the id.
    vi.mocked(planRenameId).mockReturnValue(new Promise(() => {}));
    await show({ selected: { kind: "flag", number: 0x94 } });
    await click(button("Umbenennen…"));
    const rename = document.querySelector<HTMLElement>("[data-rename-id-dialog]")!;
    expect(rename.querySelector<HTMLInputElement>('input[aria-label="Neue ID"]')?.value).toBe("FLAG_HAT_MEDAILLE");
  });

  it("says_what_the_list_lacks_in_the_dspre_layout_and_without_adopted_scripts", async () => {
    flags = flagList("flag", FLAGS.entries, { text_project: false, scripts_indexed: false });
    await show();
    const notices = [...surface().querySelectorAll("[role=status]")].map((n) => n.textContent);
    expect(notices).toEqual([
      "Eigene Namen gibt es im Textprojekt: Technik › Textprojekt › „Als Textprojekt einrichten…“.",
      "Scripts sind noch nicht übernommen: Script-Verweise fehlen im Index, „frei“ ist darum nur ein Hinweis.",
    ]);
    // Without a project: nothing to read.
    vi.mocked(listFlags).mockClear();
    await show({ project: null });
    expect(listFlags).not.toHaveBeenCalled();
    expect(surface().textContent).toContain("Kein Projekt geöffnet.");
  });

  it("selection_follows_the_nav_target", async () => {
    // A link to a variable: the variables show, with the number chosen.
    await show({ selected: { kind: "var", number: 0x4000 } });
    expect(chip("Art", "Variablen").getAttribute("aria-pressed")).toBe("true");
    expect(row(0x4000).getAttribute("aria-selected")).toBe("true");
    expect(flagUsage).toHaveBeenLastCalledWith("var", 0x4000);
    expect(detail().querySelector("h2")?.textContent).toBe("VAR_MAP_LOCAL_0x00");
    // A filter that hides the next target is lifted when a link leads there.
    typeInto(search(), "nichts davon");
    await flush();
    await show({ selected: { kind: "flag", number: 0x94 } });
    expect(chip("Art", "Flags").getAttribute("aria-pressed")).toBe("true");
    expect(search().value).toBe("");
    expect(row(0x94).getAttribute("aria-selected")).toBe("true");
    expect(detail().querySelector("h2")?.textContent).toBe("FLAG_HAT_MEDAILLE");
  });

  it("names_the_count_in_the_subtitle", () => {
    expect(flagsSubtitle(null)).toBe("Wird gelesen …");
    expect(flagsSubtitle(flagList("flag", Array.from({ length: 4097 }, (_, i) => flagEntry(i + 1)), { pool_free: 135 }))).toBe("4.097 Flags · 135 frei");
    expect(flagsSubtitle(flagList("var", [flagEntry(0x4000)], { pool_free: 1 }))).toBe("1 Variable · 1 frei");
  });
});
