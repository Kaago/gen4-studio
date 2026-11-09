// @vitest-environment jsdom
import i18next from "i18next";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { planDelete } from "../api";
import { DeleteDialog } from "./DeleteDialog";
import { TRAINER_901, deletePlan, figureRef } from "./deleteTesting";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  planDelete: vi.fn(),
  deleteEntity: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DeleteDialog in English", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(async () => {
    await i18next.changeLanguage("en");
    vi.mocked(planDelete).mockReset().mockResolvedValue(deletePlan({ references: [figureRef(2), figureRef(5)] }));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    await i18next.changeLanguage("de");
  });

  it("says the entity cannot be deleted while references point at it", async () => {
    await act(async () => root.render(<DeleteDialog target={TRAINER_901} onClose={() => {}} onDeleted={() => {}} goTo={() => {}} />));
    const dialog = document.querySelector<HTMLElement>("[role=dialog]");
    const alert = dialog?.querySelector<HTMLElement>("[role=alert]")?.textContent ?? "";
    expect(alert).toContain("cannot be deleted");
    expect(alert).toContain("references");
    expect(dialog?.querySelector("h2")?.textContent).toBe("Delete Trainer 0901");
    expect(dialog?.querySelector("[role=listbox]")?.getAttribute("aria-label")).toBe("References");
    expect([...(dialog?.querySelectorAll("button") ?? [])].map((b) => b.textContent).filter((t) => t !== "")).toEqual(["Cancel", "Delete"]);
  });
});
