// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SoundNameDto } from "../../api";
import { typeInSearch } from "../../ui/primitives/testUtils";
import type { FieldSpec } from "./fields";
import { MusicField } from "./MusicField";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const spec: FieldSpec = { key: "music_day_id", label: "Musik (Tag)", kind: "u16", pick: "music" };
const SOUNDS: SoundNameDto[] = [
  { id: 1000, name: "SEQ_TOWN01", used_in: "Zweiblattdorf" },
  { id: 1001, name: "SEQ_TOWN02", used_in: null },
  { id: 1052, name: "SEQ_ROAD_A_D", used_in: "Route 201 (Tag)" },
];

describe("MusicField", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  const show = (value: number, sounds: readonly SoundNameDto[] | null = SOUNDS) => act(() => root.render(<MusicField spec={spec} value={value} sounds={sounds} onCommit={onCommit} />));
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
  const choose = () => act(async () => void container.querySelector<HTMLButtonElement>("button[aria-label='Musik (Tag) wählen…']")!.click());
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onCommit.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("picks a piece by name and shows used_in", async () => {
    show(1000);
    // The row names the piece and where the game plays it; no number in Erzählen.
    const row = container.querySelector<HTMLElement>("[data-music-field='music_day_id']")!;
    expect(row.textContent).toContain("SEQ_TOWN01");
    expect(row.textContent).toContain("Zweiblattdorf");
    expect(row.textContent).not.toContain("1000");
    await choose();
    // Each piece by its name, with where it plays as the description and its number as the technical part.
    expect(options()).toHaveLength(3);
    expect(options()[0]!.textContent).toContain("SEQ_TOWN01");
    expect(options()[0]!.textContent).toContain("Zweiblattdorf");
    expect(options()[0]!.textContent).toContain("1000");
    expect(options()[1]!.textContent).toContain("SEQ_TOWN02");
    // Found by name and by where it plays.
    typeInSearch(document.querySelector<HTMLInputElement>("[role=dialog] input")!, "Route 201");
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining("SEQ_ROAD_A_D")]);
    await act(async () => options()[0]!.click());
    expect(onCommit).toHaveBeenCalledWith(spec, 1052);
  });

  it("names_a_piece_the_table_does_not_know_by_its_number_and_waits_for_the_table", async () => {
    show(7);
    expect(container.querySelector("[data-music-field]")?.textContent).toContain("Stück 7");
    show(7, null);
    expect(container.querySelector("[data-music-field]")?.textContent).toContain("Stück 7");
    await choose();
    expect(options()).toHaveLength(0);
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Lädt …");
  });
});
