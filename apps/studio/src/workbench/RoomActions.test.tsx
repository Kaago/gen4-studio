// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, WorldMeta } from "../api";
import { ownedMeta, renamedMeta, RoomActions } from "./RoomActions";
import type { Room } from "./worldModel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const meta = (patch: Partial<WorldMeta> = {}): WorldMeta => ({ version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {}, ...patch });

describe("renamedMeta and ownedMeta", () => {
  it("names_a_room_and_an_empty_name_gives_the_suggestion_back", () => {
    const named = renamedMeta(meta(), { kind: "room", id: 3 }, "  Labor ");
    expect(named.roomNames).toEqual({ "3": "Labor" });
    expect(renamedMeta(named, { kind: "room", id: 3 }, " ").roomNames).toEqual({});
  });

  it("names_a_world_map_per_half_and_replaces_its_earlier_name", () => {
    let m = renamedMeta(meta(), { kind: "worldMap", matrix: 0, filter: "kanto" }, "Kanto-Region");
    m = renamedMeta(m, { kind: "worldMap", matrix: 0, filter: null }, "Sinnoh");
    m = renamedMeta(m, { kind: "worldMap", matrix: 0, filter: "kanto" }, "Kanto");
    expect(m.worldMaps).toEqual([
      { matrix: 0, name: "Sinnoh" },
      { matrix: 0, filter: "kanto", name: "Kanto" },
    ]);
    expect(renamedMeta(m, { kind: "worldMap", matrix: 0, filter: null }, "").worldMaps).toEqual([{ matrix: 0, filter: "kanto", name: "Kanto" }]);
  });

  it("assigns_a_room_and_takes_it_back_without_changing_the_input", () => {
    const before = meta();
    const owned = ownedMeta(before, 4, "w0:3");
    expect(owned.roomOwner).toEqual({ "4": "w0:3" });
    expect(before.roomOwner).toEqual({});
    expect(ownedMeta(owned, 4, null).roomOwner).toEqual({});
  });
});

describe("RoomActions", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const room = (manual: boolean): Room => ({
    header: { id: 3, internal_name: "T01R0101" } as HeaderSummary,
    name: "Haus 1",
    suggested: "Haus 1",
    placeKey: "Zweiblattdorf",
    placeName: "Zweiblattdorf",
    manual,
    shared: 0,
  });
  const open = () =>
    act(() => {
      const trigger = container.querySelector<HTMLElement>('[aria-label="Weitere Aktionen für Haus 1"]')!;
      trigger.click();
    });
  const items = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].map((e) => e.textContent);

  it("offers_the_reset_only_for_a_room_assigned_by_hand", () => {
    const onOwner = vi.fn();
    act(() => root.render(<RoomActions room={room(false)} areaKey="w0:2" areas={[]} onRename={() => {}} onOwner={onOwner} />));
    open();
    expect(items()).toEqual(["Umbenennen…", "Gehört zu Gebiet…"]);
    act(() => root.render(<RoomActions room={room(true)} areaKey="w0:2" areas={[]} onRename={() => {}} onOwner={onOwner} />));
    expect(items()).toEqual(["Umbenennen…", "Gehört zu Gebiet…", "Zuordnung zurücksetzen"]);
    act(() => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')][2].click());
    expect(onOwner).toHaveBeenCalledWith(3, null);
  });

  it("does_not_let_a_click_reach_the_row_around_it", () => {
    const onRow = vi.fn();
    act(() =>
      root.render(
        <div onClick={onRow}>
          <RoomActions room={room(false)} areaKey="w0:2" areas={[]} onRename={() => {}} onOwner={() => {}} />
        </div>,
      ),
    );
    open();
    act(() => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')][0].click());
    act(() => [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((b) => b.textContent === "Abbrechen")!.click());
    expect(onRow).not.toHaveBeenCalled();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });
});
