// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { defaultProjectParent } from "../api";
import { NewProjectWizard } from "./NewProjectWizard";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), inspectRom: vi.fn(), defaultProjectParent: vi.fn(), checkProjectDest: vi.fn(), createProject: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("NewProjectWizard in English", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(async () => {
    await i18next.changeLanguage("en");
    vi.mocked(defaultProjectParent).mockReset().mockResolvedValue({ parent: "/roms", dspre_suffix: false });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    await i18next.changeLanguage("de");
  });

  it("names the wizard and its template step in English", async () => {
    await act(async () => root.render(<NewProjectWizard onCancel={() => {}} onOpen={() => {}} />));
    const text = container.textContent ?? "";
    expect(text).toContain("New game from ROM");
    expect(text).toContain("Template");
    expect(text).toContain("Choose file…");
  });
});
