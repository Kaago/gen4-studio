// @vitest-environment jsdom
import { act } from "react";
import type { ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { releases } from "../api";
import { releasesOf } from "../publish/publishTesting";
import { ShellProvider } from "../ui/frame/ShellContext";
import type { WizardRequest } from "../workbench/WorkbenchContext";
import { WizardHost } from "./WizardHost";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), releases: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("WizardHost", () => {
  let container: HTMLElement;
  let root: Root;
  const publish = { gitOwn: true, onPublish: vi.fn(), onReveal: vi.fn(), onCommit: vi.fn(), onClose: vi.fn() };

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(releases).mockReset();
    for (const fn of Object.values(publish)) if (typeof fn === "function") fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function props(request: WizardRequest | null): ComponentProps<typeof WizardHost> {
    return {
      wizards: { request, close: vi.fn(), created: vi.fn() },
      headers: [],
      matrices: [],
      world: null,
      matricesLoaded: true,
      matricesFailed: false,
      links: [],
      counts: {},
      selected: null,
      shownMatrix: null,
      lastTile: () => null,
      onHeaderPatch: async () => {},
      onError: vi.fn(),
      onPlay: vi.fn(),
      scripts: null,
      onAdoptScripts: vi.fn(),
      unsaved: 0,
      onSave: async () => {},
      onRegionApplied: async () => {},
      publish,
    };
  }
  const show = async (request: WizardRequest | null) => {
    await act(async () =>
      root.render(
        <ShellProvider>
          <WizardHost {...props(request)} />
        </ShellProvider>,
      ),
    );
    await flush();
  };
  const field = (label: string) => container.querySelector<HTMLInputElement>(`[aria-label="${label}"]`);

  it("publish_request_renders_the_publish_wizard", async () => {
    vi.mocked(releases).mockResolvedValue(releasesOf({ next_version: "0.2.0" }));
    await show({ kind: "publish" });
    // The host reads the releases list when the wizard opens: the proposal is in the field.
    expect(releases).toHaveBeenCalledOnce();
    expect(container.querySelector('[role="dialog"]')?.getAttribute("aria-labelledby")).toBeTruthy();
    expect(container.textContent).toContain("Veröffentlichen");
    expect(field("Version")?.value).toBe("0.2.0");
    act(() => [...container.querySelectorAll("button")].find((b) => b.textContent === "Abbrechen")!.click());
    expect(publish.onClose).toHaveBeenCalledWith(false);

    // A list that cannot be read: its sentence, nothing to go on with.
    vi.mocked(releases).mockRejectedValue({ code: "releases_broken", message: "Die Liste der Veröffentlichungen ist nicht lesbar: kaputt", asset: null });
    await show(null);
    await show({ kind: "publish" });
    expect(container.querySelector("[role=alert]")?.textContent).toBe("Die Liste der Veröffentlichungen ist nicht lesbar: kaputt");
    expect([...container.querySelectorAll("button")].find((b) => b.textContent === "Weiter")?.disabled).toBe(true);
  });
});
