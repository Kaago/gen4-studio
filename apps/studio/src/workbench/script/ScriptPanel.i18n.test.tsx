// @vitest-environment jsdom
import type { DockviewApi, IDockviewPanelProps } from "dockview-react";
import i18next from "i18next";
import { act, useState } from "react";
import type { FunctionComponent } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectInfo } from "../../api";
import { lspStart, scriptsStatus } from "../../api";
import { createEditBus } from "../../bus/editBus";
import { resetSpy, withReset } from "../../original/resetTesting";
import { Deck } from "../../ui/frame/Deck";
import { ShellProvider } from "../../ui/frame/ShellContext";
import type { PanelId } from "../layout";
import { ProblemsStore } from "../problems/problems";
import { ScriptPanel } from "./ScriptPanel";
import { useScripts } from "./useScripts";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  scriptsStatus: vi.fn(),
  lspStart: vi.fn(),
  lspSend: vi.fn(),
  lspStop: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const project: ProjectInfo = { root: "/a", game_code: "CPUE", family: "Pt", language: "en", header_count: 3 };

/** The script panel as the world's dock holds it: a project whose scripts are not adopted yet. */
function ScriptHost() {
  const [bus] = useState(createEditBus);
  const [problems] = useState(() => new ProblemsStore());
  const { api: scripts } = useScripts({ info: project, bus, problems, onError: () => {}, onSaved: () => {} });
  return <ScriptPanel context={{ project, scripts, problems, bus, depth: "bauen", onError: () => {} }} />;
}
const ScriptSlot: FunctionComponent<IDockviewPanelProps> = () => withReset(resetSpy(), <ScriptHost />);

const Empty: FunctionComponent<IDockviewPanelProps> = () => null;
const others: PanelId[] = ["world", "matrix", "worldmap", "inspector", "problems"];
const WELT = { ...Object.fromEntries(others.map((id) => [id, Empty])), script: ScriptSlot };

describe("script editor frame in English", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(async () => {
    localStorage.clear();
    vi.mocked(scriptsStatus).mockReset().mockResolvedValue({ adopted: false, file_count: 4 });
    vi.mocked(lspStart).mockReset().mockRejectedValue({ code: "tool_missing", message: "rotom-lsp missing", asset: null });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    await i18next.changeLanguage("de");
  });

  it("shows the adoption box and the problems tab in English", async () => {
    const dockRef = { current: null as DockviewApi | null };
    await act(async () =>
      root.render(
        <ShellProvider initialArea="welt">
          <Deck area="welt" depth="bauen" components={WELT} dockRef={dockRef} hinge={<div />} />
        </ShellProvider>,
      ),
    );
    await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    const tabs = [...container.querySelectorAll('[role="tab"]')].map((t) => t.textContent);
    expect(tabs).toContain("Problems");
    expect(container.textContent).toContain("Adopt scripts");
  });
});
