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

  it("offers_exactly_the_six_parts_of_the_map_file_not_the_textures", async () => {
    await act(async () => root.render(<ExportPartDialog map={5} open onClose={() => {}} />));
    const group = dialog().querySelector("[aria-label='Teil']")!;
    const labels = [...group.querySelectorAll("button")].map((b) => b.textContent);
    expect(labels).toEqual(["Modell", "Kollision", "Gebäude", "BDHC (Laufhöhen)", "Hintergrundklang", "Ganze Karte"]);
    expect(dialog().textContent).not.toContain("Texturen");
  });

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

  const pathField = () => dialog().querySelector<HTMLInputElement>("input[aria-label='Pfad']")!;
  async function typePath(text: string) {
    await act(async () => void pathField().focus());
    typeInto(pathField(), text);
    await act(async () => void pathField().blur());
  }

  it("names_the_part_it_wrote_and_moves_the_path_to_the_ending_of_the_next_part", async () => {
    await act(async () => root.render(<ExportPartDialog map={6} open onClose={() => {}} />));
    await typePath("/out/export/karte6.nsbmd");
    await act(async () => button("Exportieren").click());
    expect(exportMapPart).toHaveBeenLastCalledWith(6, "model", "/out/export/karte6.nsbmd");
    expect(dialog().textContent).toContain("Modell exportiert nach /out/export/karte6.nsbmd");
    // Another part: the notice of the model goes, the path takes the ending of the permissions.
    await act(async () => button("Kollision").click());
    expect(dialog().textContent).not.toContain("exportiert nach");
    expect(pathField().value).toBe("/out/export/karte6.per");
    await act(async () => button("Exportieren").click());
    expect(exportMapPart).toHaveBeenLastCalledWith(6, "permissions", "/out/export/karte6.per");
    expect(dialog().textContent).toContain("Kollision exportiert nach /out/export/karte6.per");
    // A new path clears the notice too: it spoke of the old one.
    await typePath("/out/export/k6.per");
    expect(dialog().textContent).not.toContain("exportiert nach");
  });

  it("proposes_the_default_name_for_a_folder_and_keeps_an_ending_it_does_not_know", async () => {
    await act(async () => root.render(<ExportPartDialog map={6} open onClose={() => {}} />));
    // Empty: the placeholder shows the default name with the part's ending.
    await act(async () => button("Kollision").click());
    expect(pathField().value).toBe("");
    expect(pathField().placeholder).toBe("/Pfad/karte_0006.per");
    await typePath("/out/export/");
    await act(async () => button("Gebäude").click());
    expect(pathField().value).toBe("/out/export/karte_0006.bld");
    await typePath("/out/export/karte6.v2");
    await act(async () => button("Ganze Karte").click());
    expect(pathField().value).toBe("/out/export/karte6.v2.bin");
  });

  it("says_in_german_that_the_folder_is_missing_and_how_to_go_on", async () => {
    // The bridge checks the folder first; the app's kernel fails when it writes. Both say it in English.
    for (const message of [
      "Pfad nicht lesbar: /out/neu/karte6.nsbmd: No such file or directory (os error 2)",
      "/out/neu/karte6.nsbmd: No such file or directory (os error 2)",
    ]) {
      vi.mocked(exportMapPart).mockRejectedValueOnce({ code: "error", message, asset: null });
      await act(async () => root.render(<ExportPartDialog map={6} open onClose={() => {}} />));
      await typePath("/out/neu/karte6.nsbmd");
      await act(async () => button("Exportieren").click());
      const alert = dialog().querySelector("[role=alert]")!;
      expect(alert.textContent).toBe("Den Ordner /out/neu gibt es nicht. Lege ihn an oder wähle einen anderen Pfad.");
      await act(async () => root.render(<ExportPartDialog map={6} open={false} onClose={() => {}} />));
    }
  });

  it("shows_any_other_refusal_as_the_backend_says_it", async () => {
    vi.mocked(exportMapPart).mockRejectedValueOnce({ code: "error", message: "Pfad liegt außerhalb der erlaubten Ordner: /etc/k.nsbmd", asset: null });
    await act(async () => root.render(<ExportPartDialog map={6} open onClose={() => {}} />));
    await typePath("/etc/k.nsbmd");
    await act(async () => button("Exportieren").click());
    expect(dialog().querySelector("[role=alert]")!.textContent).toBe("Pfad liegt außerhalb der erlaubten Ordner: /etc/k.nsbmd");
  });
});
