// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignBase, baseStatus, createPatch, registerBaseRom } from "../api";
import type { BaseStatus } from "../api";
import { found, mark, missing, patchDto } from "./baseTesting";
import { useBase } from "./useBase";
import type { BaseApi } from "./useBase";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  baseStatus: vi.fn(),
  checkBase: vi.fn(),
  assignBase: vi.fn(),
  registerBaseRom: vi.fn(),
  createPatch: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useBase", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: BaseApi;

  function Probe() {
    shown = useBase();
    return null;
  }

  beforeEach(() => {
    vi.mocked(baseStatus).mockReset().mockResolvedValue({ state: "unassigned", suggestion: null });
    vi.mocked(assignBase).mockReset();
    vi.mocked(registerBaseRom).mockReset();
    vi.mocked(createPatch).mockReset();
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("reads_the_status_once_per_project", async () => {
    await act(async () => root.render(<Probe key="/a" />));
    expect(shown.status).toEqual({ state: "unassigned", suggestion: null });
    await act(async () => root.render(<Probe key="/a" />));
    expect(baseStatus).toHaveBeenCalledOnce();
    // TechnikArea keys the hook by project: another project reads again.
    vi.mocked(baseStatus).mockResolvedValue(found());
    await act(async () => root.render(<Probe key="/b" />));
    expect(baseStatus).toHaveBeenCalledTimes(2);
    expect(shown.status?.state).toBe("found");
  });

  it("assigns_and_reads_the_status_again", async () => {
    const after = found();
    vi.mocked(assignBase).mockResolvedValue(after);
    await act(async () => root.render(<Probe />));
    let pending: Promise<void> = Promise.resolve();
    act(() => {
      pending = shown.assign("/roms/Sinnoh-Remix.nds");
    });
    expect(shown.busy).toBe("assign");
    await act(async () => pending);
    expect(assignBase).toHaveBeenCalledWith("/roms/Sinnoh-Remix.nds");
    expect(shown.status).toEqual(after);
    expect(shown.busy).toBeNull();
    expect(shown.error).toBeNull();

    // refresh reads the status from the backend again.
    vi.mocked(baseStatus).mockResolvedValue(missing());
    await act(async () => shown.refresh());
    expect(shown.status?.state).toBe("missing");
  });

  it("keeps_the_last_patch", async () => {
    vi.mocked(baseStatus).mockResolvedValue(found());
    vi.mocked(createPatch).mockResolvedValue(patchDto());
    await act(async () => root.render(<Probe />));
    expect(shown.lastPatch).toBeNull();
    let pending: Promise<void> = Promise.resolve();
    act(() => {
      pending = shown.createPatch();
    });
    expect(shown.busy).toBe("patch");
    await act(async () => pending);
    expect(shown.lastPatch).toEqual(patchDto());
    expect(shown.busy).toBeNull();
    // Reading the status again keeps it.
    await act(async () => shown.refresh());
    expect(shown.lastPatch).toEqual(patchDto());

    // A failed patch says why.
    vi.mocked(createPatch).mockRejectedValue({ code: "base_missing", message: "Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).", asset: null, problems: [] });
    await act(async () => shown.createPatch());
    expect(shown.error).toBe("Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).");
  });

  it("a_refused_register_keeps_the_status_and_says_why", async () => {
    const before: BaseStatus = missing();
    vi.mocked(baseStatus).mockResolvedValue(before);
    vi.mocked(registerBaseRom).mockRejectedValue({ code: "error", message: "Das ist eine andere ROM (SHA-1 13b6853a…), gesucht ist Platin DE (Testprojekt) (d40f19ac…).", asset: null, problems: [] });
    await act(async () => root.render(<Probe />));
    await act(async () => shown.register("/roms/HeartGold.nds"));
    expect(registerBaseRom).toHaveBeenCalledWith("/roms/HeartGold.nds");
    expect(shown.status).toEqual(before);
    expect(shown.error).toBe("Das ist eine andere ROM (SHA-1 13b6853a…), gesucht ist Platin DE (Testprojekt) (d40f19ac…).");
    expect(shown.busy).toBeNull();

    // A register that finds it clears the reason.
    vi.mocked(registerBaseRom).mockResolvedValue(found({ path: "/roms/renamed.nds", source: "known", mark: mark() }));
    await act(async () => shown.register("/roms/renamed.nds"));
    expect(shown.status?.state).toBe("found");
    expect(shown.error).toBeNull();
  });
});
