// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { freeFlag, listFlags } from "../api";
import type { FlagKind, FlagListDto, ProjectInfo } from "../api";
import { typeInSearch } from "../ui/primitives/testUtils";
import { FLAGS, flagEntry, flagList } from "./flagTesting";
import { FlagPicker } from "./FlagPicker";
import { invalidateFlagLists } from "./useFlagList";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listFlags: vi.fn(), freeFlag: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });
const VARS: FlagListDto = flagList("var", [flagEntry(0x4000, { name: "VAR_MAP_LOCAL_0x00", area: "map_local" }), flagEntry(0x4030, { free: true })]);

describe("FlagPicker", () => {
  let container: HTMLElement;
  let root: Root;
  const onPick = vi.fn();
  const onOpenChange = vi.fn();
  const onShowInDirectory = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(listFlags).mockReset().mockImplementation(async (kind) => (kind === "flag" ? FLAGS : VARS));
    vi.mocked(freeFlag).mockReset();
    invalidateFlagLists();
    for (const fn of [onPick, onOpenChange, onShowInDirectory]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  type Props = Partial<Parameters<typeof FlagPicker>[0]>;
  const show = async (over: Props = {}, kind: FlagKind = "flag") => {
    await act(async () =>
      root.render(<FlagPicker open kind={kind} value={0x94} onOpenChange={onOpenChange} onPick={onPick} project={project()} revision={0} {...over} />),
    );
    await flush();
  };
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const button = (text: string) => [...dialog().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text);
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());
  /** The reason a locked button names for assistive technology (and in its tooltip). */
  const reason = (el: HTMLElement | undefined) => document.getElementById(el?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("the_picker_offers_none_and_keeps_zero", async () => {
    await show();
    expect(dialog().textContent).toContain("Flag wählen");
    expect(document.querySelector<HTMLInputElement>("input[role=combobox]")!.placeholder).toBe("Name oder Nummer");
    expect(dialog().querySelector("[data-flag-picker]")).not.toBeNull();
    expect(options()[0]!.textContent).toContain("Keins (0)");
    expect(options().map((o) => o.textContent)).toHaveLength(6);
    // 0 is a choice like any other: it reaches the owner as 0, not as "nothing picked".
    await click(options()[0]);
    expect(onPick).toHaveBeenCalledWith(0);
    expect(onOpenChange).toHaveBeenCalledWith(false);
    // A search keeps „Keins (0)“ first; a pick of a named number gives its number. An own name is bold, a pret name is not.
    await show();
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "medaille");
    expect(options().map((o) => o.textContent?.startsWith("Keins (0)"))).toEqual([true, false]);
    expect(options()[1]!.querySelector("[class*=strong]")?.textContent).toBe("FLAG_HAT_MEDAILLE");
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "0x0096");
    expect(options()[0]!.textContent).toContain("Keins (0)");
    expect(options()[1]!.textContent).toContain("FLAG_UNUSED_0x0096");
    expect(options()[1]!.querySelector("[class*=strong]")).toBeNull();
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "medaille");
    await click(options().find((o) => o.textContent?.includes("FLAG_HAT_MEDAILLE")));
    expect(onPick).toHaveBeenLastCalledWith(0x94);
  });

  it("filters_by_area", async () => {
    await show();
    await click(button("Trainer"));
    expect(options().map((o) => o.textContent?.includes("FLAG_TRAINER_0"))).toEqual([false, true]);
    await click(button("Alle"));
    expect(options()).toHaveLength(6);
    // Only the areas the list has are offered.
    expect(button("Tagesflags")).toBeUndefined();
  });

  it("free_flag_takes_the_pool_number", async () => {
    vi.mocked(freeFlag).mockResolvedValue({ kind: "flag", number: 0x173, name: null, pool_free: 3, scripts_indexed: true });
    await show();
    await click(button("Freies Flag nehmen"));
    await flush();
    expect(freeFlag).toHaveBeenCalledWith("flag");
    expect(onPick).toHaveBeenCalledWith(0x173);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("a_refused_free_flag_shows_the_kernel_sentence", async () => {
    vi.mocked(freeFlag).mockRejectedValue({ code: "pool_empty", message: "Kein freies Flag im allgemeinen Bereich.", asset: null });
    await show();
    await click(button("Freies Flag nehmen"));
    await flush();
    expect(onPick).not.toHaveBeenCalled();
    expect(dialog().textContent).toContain("Kein freies Flag im allgemeinen Bereich.");
    expect(button("Freies Flag nehmen")!.getAttribute("aria-disabled")).toBe("true");
  });

  it("empty_pool_locks_the_footer_with_the_reason", async () => {
    vi.mocked(listFlags).mockImplementation(async (kind) => (kind === "flag" ? { ...FLAGS, pool_free: 0 } : { ...VARS, pool_free: 0 }));
    await show();
    const take = button("Freies Flag nehmen")!;
    // Locked but focusable: the reason is its description and its tooltip.
    expect(take.disabled).toBe(false);
    expect(take.getAttribute("aria-disabled")).toBe("true");
    expect(reason(take)).toBe("Kein freies Flag im allgemeinen Bereich.");
    await click(take);
    expect(freeFlag).not.toHaveBeenCalled();
    expect(onPick).not.toHaveBeenCalled();
    // Variables: their own words.
    await show({ kind: "var", value: 0 }, "var");
    expect(dialog().textContent).toContain("Variable wählen");
    expect(reason(button("Freie Variable nehmen"))).toBe("Keine freie Variable im allgemeinen Bereich.");
  });

  it("scripts_not_adopted_lock_the_free_flag_with_the_reason", async () => {
    vi.mocked(listFlags).mockResolvedValue({ ...FLAGS, scripts_indexed: false });
    await show();
    const take = button("Freies Flag nehmen")!;
    expect(take.getAttribute("aria-disabled")).toBe("true");
    expect(reason(take)).toBe("Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche Flags Scripts benutzen.");
    await click(take);
    expect(freeFlag).not.toHaveBeenCalled();
  });

  it("every_opening_reads_anew_so_an_adoption_unlocks_the_free_flag", async () => {
    // Read before the scripts were adopted: locked.
    vi.mocked(listFlags).mockResolvedValue({ ...FLAGS, scripts_indexed: false });
    const p = project();
    await show({ project: p });
    expect(button("Freies Flag nehmen")!.getAttribute("aria-disabled")).toBe("true");
    await show({ project: p, open: false });
    // Adopted since: the same revision, but the next opening reads again and the button is free.
    vi.mocked(listFlags).mockResolvedValue(FLAGS);
    await show({ project: p });
    expect(listFlags).toHaveBeenCalledTimes(2);
    expect(button("Freies Flag nehmen")!.getAttribute("aria-disabled")).toBeNull();
  });

  it("show_in_directory_calls_back", async () => {
    // Without a way to the directory there is no such button.
    await show();
    expect(button("Im Verzeichnis zeigen")).toBeUndefined();
    await show({ onShowInDirectory });
    await click(button("Im Verzeichnis zeigen"));
    // The picker closes first; the list opens on the tick after, so the dialog's focus return does not run into it.
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onShowInDirectory).not.toHaveBeenCalled();
    await flush();
    expect(onShowInDirectory).toHaveBeenCalledWith(0x94);
  });

  it("reads_nothing_while_closed", async () => {
    await show({ open: false });
    expect(listFlags).not.toHaveBeenCalled();
  });
});
