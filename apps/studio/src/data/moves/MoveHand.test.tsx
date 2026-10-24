// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataRaw } from "../../api";
import type { AssetId, DataDocDto } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { openSelect, pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { movesHarness, wingAttackDoc, wireApi } from "./movesTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("MoveHand", () => {
  let h: ReturnType<typeof movesHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = movesHarness();
  });
  afterEach(() => h.unmount());

  const columns = () => [...h.hand().querySelectorAll<HTMLElement>("section[aria-label]")].map((s) => s.getAttribute("aria-label"));
  const commit = async (label: string, text: string) => {
    const input = h.field(label)!;
    typeInto(input, text);
    pressKey(input, "Enter");
    await flush();
  };

  it("has_the_columns_and_the_raw_data_only_while_tinkering", async () => {
    await h.open(17, "bauen");
    expect(h.hand().querySelector('[aria-label="Flügelschlag bearbeiten"]')?.getAttribute("data-move")).toBe("17");
    expect(columns()).toEqual(["Grundwerte", "Verhalten"]);
    expect(dataRaw).not.toHaveBeenCalled();
    h.unmount();
    h = movesHarness();
    await h.open(17, "tuefteln");
    expect(columns()).toEqual(["Grundwerte", "Verhalten", "Rohdaten"]);
  });

  it("commits_a_changed_power_as_one_labelled_edit_and_tells_the_others", async () => {
    await h.open();
    const heard: EditEvent[] = [];
    h.bus.subscribe("dataChanged", (e) => heard.push(e));
    vi.mocked(dataGet).mockClear();
    expect(h.field("Stärke")?.value).toBe("60");
    await commit("Stärke", "65");
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "power", value: 65 } }, "Flügelschlag: Stärke 60 → 65");
    expect(heard).toEqual([{ topic: "dataChanged", kind: "move", index: 17 }]);
    expect(h.calls.onEdited).toHaveBeenCalled();
    expect(dataGet).toHaveBeenCalledWith("move", 17);
  });

  it("takes_a_negative_priority_and_refuses_one_outside_minus_7_to_5", async () => {
    await h.open();
    await commit("Priorität", "-1");
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "priority", value: -1 } }, "Flügelschlag: Priorität 0 → \u22121");
    vi.mocked(dataEdit).mockClear();
    await commit("Priorität", "-8");
    await commit("Priorität", "6");
    expect(dataEdit).not.toHaveBeenCalled();
  });

  it("keeps_an_unknown_flag_bit_when_it_sets_contact", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ flags: 0x40 }));
    await h.open();
    const contact = h.checkbox("Macht Kontakt")!;
    expect(contact.getAttribute("aria-checked")).toBe("false");
    await h.click(contact);
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "flags", value: 0x41 } }, "Flügelschlag: Macht Kontakt aus → an");
  });

  it("writes_three_quick_flag_changes_each_from_the_newest_state_without_flicker", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ flags: 0 }));
    await h.open();
    // From here every read and every write waits until the test answers it.
    const gets: ((doc: DataDocDto) => void)[] = [];
    const edits: ((changed: AssetId[]) => void)[] = [];
    vi.mocked(dataGet).mockImplementation(() => new Promise((resolve) => gets.push(resolve)));
    vi.mocked(dataEdit).mockImplementation(() => new Promise((resolve) => edits.push(resolve)));
    const answer = async (doc: DataDocDto, count = gets.length) => {
      for (const resolve of gets.splice(0, count)) resolve(doc);
      await flush();
    };
    const write = async () => {
      edits.shift()!([{ kind: "Move", index: 17 }]);
      await flush();
    };
    const checked = () => ["Macht Kontakt", "Schutzschild wirkt", "Magiemantel wirkt"].map((l) => h.checkbox(l)!.getAttribute("aria-checked"));
    const shownAll: string[][] = [];
    const look = () => shownAll.push(checked().map(String));

    await h.click(h.checkbox("Macht Kontakt"));
    look();
    await h.click(h.checkbox("Schutzschild wirkt"));
    look();
    // Edit 1 builds on the backend's 0x00.
    await answer(wingAttackDoc({ flags: 0x00 }));
    expect(vi.mocked(dataEdit).mock.calls.map((c) => c[2])).toEqual([{ Move: { field: "flags", value: 0x01 } }]);
    look();
    // Edit 1 goes through: the view and the editor read the move again (R1), and edit 2 reads it fresh.
    await write();
    expect(gets).toHaveLength(3);
    // R1 lands (the state after edit 1 only) while edit 2 waits.
    await answer(wingAttackDoc({ flags: 0x01 }), 2);
    look();
    expect(checked()).toEqual(["true", "true", "false"]);
    await h.click(h.checkbox("Magiemantel wirkt"));
    look();
    expect(checked()).toEqual(["true", "true", "true"]);
    // Edit 2 builds on 0x01, edit 3 on 0x03.
    await answer(wingAttackDoc({ flags: 0x01 }));
    await write();
    look();
    await answer(wingAttackDoc({ flags: 0x03 }));
    await write();
    look();
    await answer(wingAttackDoc({ flags: 0x07 }));
    look();
    expect(vi.mocked(dataEdit).mock.calls.map((c) => [c[2], c[3]])).toEqual([
      [{ Move: { field: "flags", value: 0x01 } }, "Flügelschlag: Macht Kontakt aus → an"],
      [{ Move: { field: "flags", value: 0x03 } }, "Flügelschlag: Schutzschild wirkt aus → an"],
      [{ Move: { field: "flags", value: 0x07 } }, "Flügelschlag: Magiemantel wirkt aus → an"],
    ]);
    // A box never went back once it was ticked.
    expect(shownAll).toEqual([
      ["true", "false", "false"],
      ["true", "true", "false"],
      ["true", "true", "false"],
      ["true", "true", "false"],
      ["true", "true", "true"],
      ["true", "true", "true"],
      ["true", "true", "true"],
      ["true", "true", "true"],
    ]);
  });

  it("labels_quick_edits_of_one_field_from_the_value_each_replaces", async () => {
    await h.open();
    const edits: ((changed: AssetId[]) => void)[] = [];
    vi.mocked(dataEdit).mockImplementation(() => new Promise((resolve) => edits.push(resolve)));
    let power = 60;
    vi.mocked(dataGet).mockImplementation(async () => wingAttackDoc({ power }));
    await commit("Stärke", "65");
    await commit("Stärke", "70");
    power = 65;
    edits.shift()!([{ kind: "Move", index: 17 }]);
    await flush();
    edits.shift()!([{ kind: "Move", index: 17 }]);
    await flush();
    expect(vi.mocked(dataEdit).mock.calls.map((c) => c[3])).toEqual(["Flügelschlag: Stärke 60 → 65", "Flügelschlag: Stärke 65 → 70"]);
  });

  it("builds_the_next_edit_on_the_backend_after_a_refused_one", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ flags: 0 }));
    vi.mocked(dataEdit).mockRejectedValueOnce({ code: "error", message: "refused", asset: null });
    await h.open();
    await h.click(h.checkbox("Macht Kontakt"));
    await h.click(h.checkbox("Schutzschild wirkt"));
    expect(vi.mocked(dataEdit).mock.calls.map((c) => c[2])).toEqual([{ Move: { field: "flags", value: 0x01 } }, { Move: { field: "flags", value: 0x02 } }]);
    expect(h.checkbox("Macht Kontakt")!.getAttribute("aria-checked")).toBe("false");
  });

  it("shows_the_six_named_flags_in_bauen_and_all_eight_bits_while_tinkering", async () => {
    await h.open(17, "bauen");
    const boxes = () => [...h.hand().querySelectorAll("[role=checkbox]")];
    expect(boxes()).toHaveLength(6);
    expect(h.hand().textContent).not.toContain("Bit 6");
    h.unmount();
    h = movesHarness();
    await h.open(17, "tuefteln");
    expect(boxes()).toHaveLength(14);
    const bit6 = h.checkbox("Bit 6, 0x40")!;
    expect(bit6.getAttribute("aria-checked")).toBe("true");
    await h.click(bit6);
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "flags", value: 0x33 } }, "Flügelschlag: Bit 6 an → aus");
  });

  it("changes_type_category_and_target_by_choice", async () => {
    await h.open();
    const select = (label: string) => h.hand().querySelector<HTMLElement>(`[aria-label="${label}"]`)!;
    openSelect(select("Typ"));
    await flush();
    expect(h.options()).toHaveLength(18);
    await h.click(h.options().find((o) => o.textContent === "Feuer"));
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "move_type", value: 10 } }, "Flügelschlag: Typ Flug → Feuer");

    const category = h.hand().querySelector<HTMLElement>('[role=group][aria-label="Kategorie"]')!;
    const special = [...category.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Speziell")!;
    await h.click(special);
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "category", value: 1 } }, "Flügelschlag: Kategorie Physisch → Speziell");

    openSelect(select("Ziel"));
    await flush();
    await h.click(h.options().find((o) => o.textContent === "Beide Gegner"));
    expect(dataEdit).toHaveBeenCalledWith("move", 17, { Move: { field: "target", value: 4 } }, "Flügelschlag: Ziel Ein gewähltes Pokémon → Beide Gegner");
  });

  it("names_the_pp_with_pp_ups", async () => {
    await h.open();
    expect(h.hand().textContent).toContain("mit AP-Plus höchstens 56");
  });

  it("shows_the_raw_bytes_with_the_last_focused_field_while_tinkering", async () => {
    await h.open(17, "tuefteln");
    expect(dataRaw).toHaveBeenCalledWith("move", 17);
    expect(h.field("Effekt-ID")?.value).toBe("0");
    expect(h.hand().textContent).toContain("pl_waza_tbl.narc");
    act(() => h.field("Stärke")!.focus());
    await flush();
    expect(h.hand().textContent).toContain("Byte 3: Stärke, 0x3C = 60");
  });

  it("shows_an_error_of_the_backend_as_a_notice", async () => {
    vi.mocked(dataEdit).mockRejectedValue({ code: "error", message: "300 does not fit power", asset: null });
    await h.open();
    await commit("Stärke", "65");
    expect(h.hand().querySelector("[role=alert]")?.textContent).toContain("300 does not fit power");
  });

  it("locks_a_move_of_the_wrong_length_and_says_why_instead_of_its_fields", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { fields: [], locked: "move 0017: 18 bytes, expected 16", raw_locked: Array.from({ length: 18 }, () => 0) }));
    await h.open();
    expect(h.hand().textContent).toContain("Attacke 0017: 18 Bytes statt 16");
    expect(h.view().textContent).toContain("Attacke 0017: 18 Bytes statt 16");
    // Its fields are not readable: none shows a value.
    expect(h.field("Stärke")).toBeNull();
    expect(h.checkbox("Macht Kontakt")).toBeNull();
  });
});
