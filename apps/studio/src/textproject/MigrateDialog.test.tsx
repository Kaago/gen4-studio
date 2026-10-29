// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { migrateTextProject } from "../api";
import type { MigrateReport } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { MigrateDialog } from "./MigrateDialog";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  migrateTextProject: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const report: MigrateReport = { from: 0, to: 1, changed: ["data/moves/0001.json"], snapshot: "/p/.g4/backup/migrate-0" };

describe("MigrateDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onCancel = vi.fn();
  const onLifted = vi.fn();

  const show = () => act(async () => root.render(<MigrateDialog schema={0} onCancel={onCancel} onLifted={onLifted} />));
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);

  beforeEach(() => {
    onCancel.mockReset();
    onLifted.mockReset();
    vi.mocked(migrateTextProject).mockReset().mockResolvedValue(report);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("cancel_closes_the_project_and_lift_migrates", async () => {
    await show();
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Textprojekt anheben?");
    expect(document.getElementById(dialog()?.getAttribute("aria-describedby") ?? "")?.textContent).toBe(
      "Dieses Textprojekt hat Schema-Version 0; diese Fassung von Gen4 Studio schreibt Version 1. Anheben sichert die geänderten Dateien vorher in .g4/backup/.",
    );
    // „Abbrechen“ (and Escape alike) closes the project: it cannot be edited unlifted.
    await act(async () => button("Abbrechen")!.click());
    expect(onCancel).toHaveBeenCalledOnce();
    pressKey(dialog()!, "Escape");
    expect(onCancel).toHaveBeenCalledTimes(2);
    expect(migrateTextProject).not.toHaveBeenCalled();

    // A failure stays in the dialog with its sentence.
    vi.mocked(migrateTextProject).mockRejectedValueOnce({ code: "error", message: "data/moves/0001.json:3:1: unbekannter Schlüssel", asset: null });
    await act(async () => button("Anheben")!.click());
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("data/moves/0001.json:3:1: unbekannter Schlüssel");
    expect(onLifted).not.toHaveBeenCalled();

    await act(async () => button("Anheben")!.click());
    expect(migrateTextProject).toHaveBeenCalledTimes(2);
    expect(onLifted).toHaveBeenCalledWith(report);
  });
});
