// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { entityIds } from "../api";
import type { FlagEntryDto, FlagUsageDto } from "../api";
import { EntityIdCache, TextProjectProvider } from "../textproject/TextProjectContext";
import { FLAGS, flagEntry, flagRef, flagUsageOf } from "./flagTesting";
import { FlagDetail } from "./FlagDetail";
import type { FlagDetailProps, FlagLive } from "./FlagDetail";
import { OUTSIDE_THE_SAVE } from "./flagWords";
import { typeInto } from "../ui/primitives/testUtils";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), entityIds: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MEDAILLE: FlagEntryDto = FLAGS.entries.find((e) => e.number === 0x94)!;
const UNUSED: FlagEntryDto = FLAGS.entries.find((e) => e.number === 0x96)!;

/** The usage of 0x94: set by two scripts and an object, checked by the object (read and write) and a level script. */
const usage94: FlagUsageDto = flagUsageOf("flag", 0x94, {
  name: "FLAG_UNUSED_0x0094",
  own_id: "FLAG_HAT_MEDAILLE",
  description: "Die Medaille ist abgeholt.",
  usage: { read: 2, write: 3, unknown: 0, sources: 3 },
  refs: [
    flagRef({ source: { kind: "Script", index: 12 }, field: "SetFlag", access: "write", label: "Script 1 · SetFlag", source_name: "Scriptdatei 0012", nav: { kind: "script", file: 12, line: 4 }, header: 411 }),
    flagRef({ access: "read_write", label: "Objekt 3 · Sichtbarkeits-Flag", nav: { kind: "event", eventFile: 1, eventKind: "object", index: 3, header: 411 } }),
    flagRef({ source: { kind: "Script", index: 40 }, field: "SetFlag", access: "write", label: "Script 2 · SetFlag", source_name: "Scriptdatei 0040", nav: { kind: "script", file: 40, line: 9 }, header: 7, header_name: "Route 201" }),
    flagRef({ source: { kind: "Event", index: 9 }, field: "script", access: "read_write", label: "Trainer 0928 · Script", source_name: "Eventdatei 0009", nav: { kind: "trainer", index: 928 }, header: 7, header_name: "Route 201" }),
  ],
});

describe("FlagDetail", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { goTo: vi.fn(), onRename: vi.fn(), onName: vi.fn(), onDescribe: vi.fn() };
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const fn of Object.values(handlers)) fn.mockReset();
    vi.mocked(entityIds).mockReset().mockResolvedValue({ 411: "MAP_T01", 7: "MAP_R201" });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (over: Partial<FlagDetailProps> = {}) =>
    act(async () =>
      root.render(
        <TextProjectProvider active ids={new EntityIdCache()} onRenamed={() => {}}>
          <FlagDetail kind="flag" number={0x94} entry={MEDAILLE} usage={usage94} busy={false} error={null} textProject readOnly={false} depth="bauen" {...handlers} {...over} />
        </TextProjectProvider>,
      ),
    );
  const detail = () => container.querySelector<HTMLElement>("[data-flag-detail]")!;
  const section = (title: string) => [...detail().querySelectorAll<HTMLElement>("[data-flag-section]")].find((s) => s.getAttribute("data-flag-section") === title) ?? null;
  const uses = (scope: HTMLElement | null) => [...(scope?.querySelectorAll<HTMLButtonElement>("[data-flag-use]") ?? [])];
  const button = (name: string) => [...detail().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  /** The reason a locked button names for assistive technology (and in its tooltip). */
  const reason = (el: HTMLElement | undefined) => document.getElementById(el?.getAttribute("aria-describedby") ?? "")?.textContent;

  it("set_and_checked_sections_with_ways", async () => {
    // Nothing chosen: the detail says what to do.
    await show({ number: null, entry: null, usage: null });
    expect(detail().textContent).toBe("Wähle links ein Flag oder eine Variable.");
    await show();
    // The head: the name, the number with its area, the description.
    expect(detail().querySelector("h2")?.textContent).toBe("FLAG_HAT_MEDAILLE");
    expect(detail().textContent).toContain("0x0094 · 148 · allgemein");
    expect(detail().textContent).toContain("Die Medaille ist abgeholt.");
    // „Wo gesetzt“: write and read_write; „Wo geprüft“: read and read_write; no „Unklar“ without unclear references.
    const set = section("Wo gesetzt")!;
    const checked = section("Wo geprüft")!;
    expect(set.textContent).toContain("Wo gesetzt");
    expect(uses(set).map((u) => u.textContent)).toEqual(["Script 1 · SetFlagScriptdatei 0012", "Objekt 3 · Sichtbarkeits-FlagEventdatei 0001", "Script 2 · SetFlagScriptdatei 0040", "Trainer 0928 · ScriptEventdatei 0009"]);
    expect(uses(checked).map((u) => u.textContent)).toEqual(["Objekt 3 · Sichtbarkeits-FlagEventdatei 0001", "Trainer 0928 · ScriptEventdatei 0009"]);
    expect(section("Unklar")).toBeNull();
    // A general flag groups by source.
    expect([...set.querySelectorAll("[data-flag-group]")].map((g) => g.getAttribute("data-flag-group"))).toEqual(["Scriptdatei 0012", "Eventdatei 0001", "Scriptdatei 0040", "Eventdatei 0009"]);
    // A trainer is a chip of the Pokémon area, the rest of the world's; a click follows the way.
    expect(uses(set)[3]!.className).toMatch(/pokemon/);
    expect(uses(set)[0]!.className).toMatch(/welt/);
    await act(async () => uses(set)[0]!.click());
    expect(handlers.goTo).toHaveBeenCalledWith({ kind: "script", file: 12, line: 4 });
    await act(async () => uses(checked)[1]!.click());
    expect(handlers.goTo).toHaveBeenLastCalledWith({ kind: "trainer", index: 928 });
    // Unclear references get their own section; empty sections say so.
    await show({
      number: 0x96,
      entry: UNUSED,
      usage: flagUsageOf("flag", 0x96, { name: "FLAG_UNUSED_0x0096", refs: [flagRef({ access: "unknown", label: "Script 3 · CheckFlagVar" })] }),
    });
    expect(uses(section("Unklar")).map((u) => u.textContent)).toEqual(["Script 3 · CheckFlagVarEventdatei 0001"]);
    expect(section("Wo gesetzt")?.textContent).toContain("Nirgends gesetzt.");
    expect(section("Wo geprüft")?.textContent).toContain("Nirgends geprüft.");
    expect(detail().textContent).toContain("Keine Beschreibung");
    // While the usage is read, and when it cannot be.
    await show({ usage: null, busy: true });
    expect(detail().textContent).toContain("Wird gelesen …");
    await show({ usage: null, error: "Der Index ist nicht lesbar." });
    expect(detail().querySelector("[role=alert]")?.textContent).toBe("Der Index ist nicht lesbar.");
  });

  it("map_local_groups_by_header", async () => {
    const local = flagUsageOf("flag", 0x20, {
      name: "FLAG_MAP_LOCAL_0x20",
      area: "map_local",
      usage: { read: 3, write: 0, unknown: 0, sources: 3 },
      refs: [
        flagRef({ label: "Objekt 1 · Sichtbarkeits-Flag", header: 411 }),
        flagRef({ source: { kind: "Event", index: 2 }, label: "Objekt 0 · Sichtbarkeits-Flag", source_name: "Eventdatei 0002", header: 7, header_name: "Route 201" }),
        flagRef({ label: "Objekt 4 · Sichtbarkeits-Flag", header: 411 }),
      ],
    });
    const entry = flagEntry(0x20, { name: "FLAG_MAP_LOCAL_0x20", area: "map_local", places: 2 });
    await show({ number: 0x20, entry, usage: local });
    const checked = section("Wo geprüft")!;
    // By header, in Bauen with the place's name and the header's number.
    expect([...checked.querySelectorAll("[data-flag-group]")].map((g) => g.querySelector("h4")?.textContent)).toEqual(["Zweiblattdorf (0411)", "Route 201 (0007)"]);
    expect(uses(checked.querySelector("[data-flag-group]") as HTMLElement).map((u) => u.textContent)).toEqual(["Objekt 1 · Sichtbarkeits-FlagEventdatei 0001", "Objekt 4 · Sichtbarkeits-FlagEventdatei 0001"]);
    // In Tüfteln with the header's id of the text project.
    await show({ number: 0x20, entry, usage: local, depth: "tuefteln" });
    await act(async () => {});
    expect([...section("Wo geprüft")!.querySelectorAll("[data-flag-group] h4")].map((h) => h.textContent)).toEqual(["MAP_T01", "MAP_R201"]);
    // Without a text project there is no id: the place's name stays.
    await show({ number: 0x20, entry, usage: local, depth: "tuefteln", textProject: false });
    await act(async () => {});
    expect([...section("Wo geprüft")!.querySelectorAll("[data-flag-group] h4")].map((h) => h.textContent)).toEqual(["Zweiblattdorf (0411)", "Route 201 (0007)"]);
  });

  it("read_without_write_shows_the_hint", async () => {
    const read = flagUsageOf("flag", 0x96, { name: "FLAG_UNUSED_0x0096", usage: { read: 1, write: 0, unknown: 0, sources: 1 }, refs: [flagRef({ access: "read" })] });
    await show({ number: 0x96, entry: UNUSED, usage: read });
    const set = section("Wo gesetzt")!;
    expect(set.textContent).toContain("Gelesen, aber nirgends gesetzt: die Engine oder ein Script über eine Variable setzt es, oder nie.");
    expect(set.textContent).not.toContain("Nirgends gesetzt.");
    // Reserved by a recipe: the head says by which.
    await show({ number: 0x96, entry: { ...UNUSED, reserved: "Vorlage „Leere Region“" }, usage: { ...read, reserved: "Vorlage „Leere Region“" } });
    expect(detail().textContent).toContain("0x0096 · 150 · allgemein · Vorlage „Leere Region“");
  });

  it("the_head_locks_naming_in_the_dspre_layout", async () => {
    const plain = flagUsageOf("flag", 0x96, { name: "FLAG_UNUSED_0x0096" });
    await show({ number: 0x96, entry: UNUSED, usage: plain, textProject: false });
    const name = button("Als eigenes Flag benennen…")!;
    // Locked but focusable, with the reason and the way as its description and tooltip.
    expect(name.getAttribute("aria-disabled")).toBe("true");
    expect(name.disabled).toBe(false);
    expect(reason(name)).toBe("Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“. (Technik › Textprojekt)");
    await act(async () => name.click());
    expect(handlers.onName).not.toHaveBeenCalled();
    expect(button("Umbenennen…")).toBeUndefined();
    // A variable says so in its button.
    await show({ kind: "var", number: 0x4030, entry: flagEntry(0x4030), usage: flagUsageOf("var", 0x4030), textProject: false });
    expect(button("Als eigene Variable benennen…")?.getAttribute("aria-disabled")).toBe("true");
    // A number outside the save cannot be named, in a text project either.
    await show({ number: 0xfff, entry: flagEntry(0xfff, { area: "outside", in_save: false }), usage: flagUsageOf("flag", 0xfff, { area: "outside", in_save: false }) });
    const outside = button("Als eigenes Flag benennen…")!;
    expect(outside.getAttribute("aria-disabled")).toBe("true");
    expect(reason(outside)).toBe("Liegt außerhalb des Spielstands.");
    // HeartGold's temporary flags and the script-local variables have their area, yet lie outside the save too.
    await show({ number: 0x4000, entry: flagEntry(0x4000, { area: "temp", in_save: false }), usage: flagUsageOf("flag", 0x4000, { area: "temp", in_save: false }) });
    expect(reason(button("Als eigenes Flag benennen…")!)).toBe("Liegt außerhalb des Spielstands.");
    await show({ kind: "var", number: 0x800c, entry: flagEntry(0x800c, { name: "VAR_RESULT", area: "script_local", in_save: false }), usage: null });
    const local = button("Als eigene Variable benennen…")!;
    expect(local.getAttribute("aria-disabled")).toBe("true");
    expect(reason(local)).toBe("Liegt außerhalb des Spielstands.");
    await act(async () => local.click());
    expect(handlers.onName).not.toHaveBeenCalled();
    // In a text project, a number of the save is named.
    await show({ number: 0x96, entry: UNUSED, usage: plain });
    const open = button("Als eigenes Flag benennen…")!;
    expect(open.hasAttribute("aria-disabled")).toBe(false);
    await act(async () => open.click());
    expect(handlers.onName).toHaveBeenCalledTimes(1);
  });

  it("own_name_offers_rename_and_description", async () => {
    await show();
    expect(button("Als eigenes Flag benennen…")).toBeUndefined();
    await act(async () => button("Umbenennen…")!.click());
    expect(handlers.onRename).toHaveBeenCalledWith("FLAG_HAT_MEDAILLE");
    await act(async () => button("Beschreibung…")!.click());
    expect(handlers.onDescribe).toHaveBeenCalledTimes(1);
    // The pret name stays in sight under the own one.
    expect(detail().textContent).toContain("FLAG_UNUSED_0x0094");
  });

  describe("live", () => {
    const onSet = vi.fn();
    beforeEach(() => onSet.mockReset());
    const live = (over: Partial<FlagLive> = {}): FlagLive => ({ value: 1, busy: false, readBack: true, onSet, ...over });
    const liveSection = () => section("Live");
    const toggle = () => liveSection()!.querySelector<HTMLElement>("[role=switch]")!;

    it("the_live_section_sets_a_flag_with_a_switch_and_a_var_with_a_field", async () => {
      // Without play mode there is no section.
      await show();
      expect(liveSection()).toBeNull();
      // A flag: a switch that shows the real value; a click asks for the other one.
      await show({ live: live({ value: 1 }) });
      const sections = [...detail().querySelectorAll("[data-flag-section]")].map((s) => s.getAttribute("data-flag-section"));
      expect(sections.indexOf("Live")).toBeLessThan(sections.indexOf("Wo gesetzt"));
      expect(toggle().getAttribute("aria-checked")).toBe("true");
      await act(async () => toggle().click());
      expect(onSet).toHaveBeenCalledWith(0);
      // A write under way: the switch waits.
      await show({ live: live({ value: 0, busy: true }) });
      expect(toggle().getAttribute("aria-checked")).toBe("false");
      expect(toggle().hasAttribute("data-disabled")).toBe(true);
      // A variable: a field with the real value and „Setzen“, which writes what was typed.
      onSet.mockReset();
      const VAR = flagEntry(0x4030, { name: "VAR_TEST" });
      await show({ kind: "var", number: 0x4030, entry: VAR, usage: null, live: live({ value: 7 }) });
      const input = liveSection()!.querySelector<HTMLInputElement>("input")!;
      expect(input.value).toBe("7");
      expect(liveSection()!.querySelector("[role=switch]")).toBeNull();
      await act(async () => void input.focus());
      typeInto(input, "1234");
      await act(async () => void input.blur());
      // Typed is not written yet.
      expect(onSet).not.toHaveBeenCalled();
      const setzen = [...liveSection()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Setzen")!;
      await act(async () => setzen.click());
      expect(onSet).toHaveBeenCalledWith(1234);
      // Written: the field shows the value the game holds, not the one asked for.
      expect(input.value).toBe("7");
      // A number outside the save takes no live value.
      onSet.mockReset();
      await show({ entry: { ...MEDAILLE, in_save: false }, usage: null, live: live() });
      expect(liveSection()!.textContent).toContain(OUTSIDE_THE_SAVE);
      expect(liveSection()!.querySelector("[role=switch]")).toBeNull();
    });

    it("warns_when_the_read_back_failed", async () => {
      await show({ live: live({ value: 0 }) });
      expect(liveSection()!.textContent).not.toContain("Wert nicht übernommen");
      await show({ live: live({ value: 0, readBack: false }) });
      const warning = liveSection()!.querySelector("[role=alert], [role=status]") ?? liveSection()!;
      expect(warning.textContent).toContain("Wert nicht übernommen, noch einmal versuchen.");
      // The switch shows what the game holds.
      expect(toggle().getAttribute("aria-checked")).toBe("false");
    });

    it("names_the_map_reload_for_flags_only", async () => {
      const RELOAD = "Sichtbarkeits-Flags wirken beim nächsten Betreten der Karte.";
      await show({ live: live() });
      expect(liveSection()!.textContent).toContain(RELOAD);
      await show({ kind: "var", number: 0x4030, entry: flagEntry(0x4030), usage: null, live: live({ value: 3 }) });
      expect(liveSection()!.textContent).not.toContain(RELOAD);
    });
  });
});
