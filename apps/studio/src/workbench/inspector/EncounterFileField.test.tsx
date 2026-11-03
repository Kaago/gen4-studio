// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList } from "../../api";
import type { DataRowDto } from "../../api";
import { EncounterFileField } from "./EncounterFileField";
import type { FieldSpec } from "./fields";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), dataList: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const file = (index: number, headers: number[], deleted = false): DataRowDto => ({ index, name: "", tech: String(index).padStart(4, "0"), locked: null, extra: { users: headers }, deleted });
const SPEC: FieldSpec = { key: "wild_pokemon", label: "Begegnungen", kind: "u16" };

describe("EncounterFileField", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onCommit.mockReset();
    vi.mocked(dataList).mockReset().mockResolvedValue([file(140, []), file(141, [], true), file(182, [])]);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("the_picker_leaves_deleted_files_out", async () => {
    await act(async () => root.render(<EncounterFileField spec={SPEC} value={140} family="Pt" headers={[]} goTo={() => {}} onCommit={onCommit} />));
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Begegnungen ändern…"]')!.click());
    await flush();
    const options = [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")].map((o) => o.textContent ?? "");
    expect(options[0]).toBe("Keine");
    expect(options.some((o) => o.includes("0140"))).toBe(true);
    expect(options.some((o) => o.includes("0182"))).toBe(true);
    expect(options.some((o) => o.includes("0141"))).toBe(false);
  });
});
