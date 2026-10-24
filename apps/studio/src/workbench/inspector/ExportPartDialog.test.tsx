// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { exportMapPart } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { ExportPartDialog } from "./ExportPartDialog";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), exportMapPart: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ExportPartDialog", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(exportMapPart).mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;

  it("writes_the_chosen_part_to_the_typed_path", async () => {
    await act(async () => root.render(<ExportPartDialog map={5} open onClose={() => {}} />));
    expect(button("Exportieren").disabled).toBe(true);
    await act(async () => button("Kollision").click());
    const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Pfad']")!;
    await act(async () => void field.focus());
    typeInto(field, "/out/k5.per");
    await act(async () => void field.blur());
    await act(async () => button("Exportieren").click());
    expect(exportMapPart).toHaveBeenCalledWith(5, "permissions", "/out/k5.per");
    expect(dialog().textContent).toContain("Kollision exportiert nach /out/k5.per");
  });
});
