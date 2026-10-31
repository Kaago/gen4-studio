// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listFlags } from "../api";
import type { ProjectInfo } from "../api";
import { typeInto } from "../ui/primitives/testUtils";
import { FLAGS } from "./flagTesting";
import { FlagField } from "./FlagField";
import { invalidateFlagLists } from "./useFlagList";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listFlags: vi.fn(), freeFlag: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

describe("FlagField", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(listFlags).mockReset().mockResolvedValue(FLAGS);
    invalidateFlagLists();
    onCommit.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  type Props = Partial<Parameters<typeof FlagField>[0]>;
  const show = async (over: Props = {}) => {
    await act(async () =>
      root.render(<FlagField kind="flag" field="hidden_flag" label="Sichtbarkeits-Flag" value={0x94} onCommit={onCommit} depth="bauen" project={project()} revision={0} {...over} />),
    );
    await flush();
  };
  const chip = () => container.querySelector<HTMLButtonElement>("[data-flag-field]")!;
  const button = (text: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text);
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");

  it("shows_the_name_and_opens_the_picker", async () => {
    await show({ hint: "Die Figur ist da, solange das Flag 0 ist." });
    expect(container.textContent).toContain("Sichtbarkeits-Flag");
    expect(container.textContent).toContain("Die Figur ist da, solange das Flag 0 ist.");
    // The own name, the number in mono.
    expect(chip().getAttribute("data-flag-field")).toBe("hidden_flag");
    expect(chip().textContent).toBe("FLAG_HAT_MEDAILLE0x0094");
    expect(dialog()).toBeNull();
    await click(chip());
    expect(dialog()?.textContent).toContain("Flag wählen");
    // The picker writes the number it is given; 0 too.
    await click([...document.querySelectorAll<HTMLElement>("[role=option]")][0]);
    expect(onCommit).toHaveBeenCalledWith(0);
    // „Wählen…“ opens it as well.
    await click(button("Wählen…"));
    expect(dialog()?.textContent).toContain("Flag wählen");
    // No number field in Bauen.
    expect(container.querySelector("input[aria-label='Sichtbarkeits-Flag']")).toBeNull();
    // 0 says „Keins“; an unnamed number its kind and number.
    await show({ value: 0 });
    expect(chip().textContent).toBe("Keins");
    await show({ value: 0x173 });
    expect(chip().textContent).toBe("Flag 0x01730x0173");
    // Locked: neither the chip nor „Wählen…“ open the picker.
    await show({ disabled: true });
    expect(chip().disabled).toBe(true);
    expect(button("Wählen…")!.disabled).toBe(true);
  });

  it("zero_reads_no_list_until_the_picker_opens", async () => {
    await show({ value: 0 });
    expect(chip().textContent).toBe("Keins");
    expect(listFlags).not.toHaveBeenCalled();
    await click(chip());
    await flush();
    expect(listFlags).toHaveBeenCalledWith("flag");
  });

  it("the_picker_is_unlocked_after_an_adoption_the_field_did_not_see", async () => {
    // The field read its name before the scripts were adopted; the picker opens at the same revision after the adoption.
    const p = project();
    vi.mocked(listFlags).mockResolvedValue({ ...FLAGS, scripts_indexed: false });
    await show({ project: p });
    vi.mocked(listFlags).mockResolvedValue(FLAGS);
    await click(chip());
    await flush();
    const take = [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Freies Flag nehmen")!;
    expect(take.getAttribute("aria-disabled")).toBeNull();
  });

  it("tinker_adds_the_number_field", async () => {
    await show({ depth: "tuefteln" });
    const input = container.querySelector<HTMLInputElement>("input[aria-label='Sichtbarkeits-Flag']")!;
    expect(input.value).toBe("148");
    // With the Dez | Hex switch.
    expect(button("Hex")).toBeDefined();
    await act(async () => void input.focus());
    typeInto(input, "371");
    await act(async () => void input.blur());
    expect(onCommit).toHaveBeenCalledWith(0x173);
  });
});
