// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignBase, baseStatus, registerBaseRom, releases } from "../api";
import type { BaseStatus } from "../api";
import { releaseOf, releasesOf } from "../publish/publishTesting";
import { found, mark, missing } from "./baseTesting";
import { useBase } from "./useBase";
import type { BaseApi, UseBaseOptions } from "./useBase";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  baseStatus: vi.fn(),
  checkBase: vi.fn(),
  assignBase: vi.fn(),
  registerBaseRom: vi.fn(),
  releases: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useBase", () => {
  let container: HTMLElement;
  let root: Root;
  let shown: BaseApi;

  function Probe(options: Partial<UseBaseOptions>) {
    shown = useBase({ project: "/a", active: true, ...options });
    return null;
  }

  beforeEach(() => {
    vi.mocked(baseStatus).mockReset().mockResolvedValue({ state: "unassigned", suggestion: null });
    vi.mocked(assignBase).mockReset();
    vi.mocked(registerBaseRom).mockReset();
    vi.mocked(releases).mockReset().mockResolvedValue(releasesOf());
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
  });

  it("reads_the_status_once_per_project", async () => {
    // Without Technik on screen nothing is read (the status may hash a ROM).
    await act(async () => root.render(<Probe active={false} />));
    expect(baseStatus).not.toHaveBeenCalled();
    await act(async () => root.render(<Probe />));
    expect(shown.status).toEqual({ state: "unassigned", suggestion: null });
    await act(async () => root.render(<Probe />));
    expect(baseStatus).toHaveBeenCalledOnce();
    // Technik shown again: read again (an agent may have assigned meanwhile).
    await act(async () => root.render(<Probe active={false} />));
    await act(async () => root.render(<Probe />));
    expect(baseStatus).toHaveBeenCalledTimes(2);
    // Another project starts from nothing and reads its own.
    vi.mocked(baseStatus).mockResolvedValue(found());
    await act(async () => root.render(<Probe project="/b" />));
    expect(baseStatus).toHaveBeenCalledTimes(3);
    expect(shown.status?.state).toBe("found");
    // No project: nothing is read.
    await act(async () => root.render(<Probe project={null} />));
    expect(shown.status).toBeNull();
    expect(baseStatus).toHaveBeenCalledTimes(3);
  });

  it("drops_an_answer_for_the_project_before", async () => {
    let answer: (status: BaseStatus) => void = () => {};
    vi.mocked(baseStatus).mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    await act(async () => root.render(<Probe />));
    vi.mocked(baseStatus).mockResolvedValue({ state: "unassigned", suggestion: null });
    await act(async () => root.render(<Probe project="/b" />));
    await act(async () => answer(found()));
    expect(shown.status).toEqual({ state: "unassigned", suggestion: null });
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

  it("reads_the_releases_with_the_status_and_again_on_request", async () => {
    // Without Technik on screen nothing is read.
    await act(async () => root.render(<Probe active={false} />));
    expect(releases).not.toHaveBeenCalled();
    expect(shown.releases).toBeNull();
    expect(shown.releasesError).toBeNull();
    // With the status: the list.
    await act(async () => root.render(<Probe />));
    expect(baseStatus).toHaveBeenCalledOnce();
    expect(releases).toHaveBeenCalledOnce();
    expect(shown.releases).toEqual(releasesOf());
    // Again on request (after a release), without reading the status.
    const after = releasesOf({ next_version: "0.2.0", releases: [releaseOf()] });
    vi.mocked(releases).mockResolvedValue(after);
    await act(async () => shown.refreshReleases());
    expect(releases).toHaveBeenCalledTimes(2);
    expect(baseStatus).toHaveBeenCalledOnce();
    expect(shown.releases).toEqual(after);
    // Reading the status again reads only the status.
    await act(async () => shown.refresh());
    expect(baseStatus).toHaveBeenCalledTimes(2);
    expect(releases).toHaveBeenCalledTimes(2);

    // A broken list: no list, its reason; the status is not touched.
    const broken = "Die Liste der Veröffentlichungen .g4/releases.json ist nicht lesbar (expected value); sie bleibt, wie sie ist.";
    vi.mocked(releases).mockRejectedValue({ code: "releases_broken", message: broken, asset: null, problems: [] });
    await act(async () => shown.refreshReleases());
    expect(shown.releases).toBeNull();
    expect(shown.releasesError).toBe(broken);
    expect(shown.error).toBeNull();
    expect(shown.status).toEqual({ state: "unassigned", suggestion: null });
    // Readable again: the reason goes.
    vi.mocked(releases).mockResolvedValue(after);
    await act(async () => shown.refreshReleases());
    expect(shown.releasesError).toBeNull();
    expect(shown.releases).toEqual(after);

    // Another project starts from nothing and reads its own; an answer for the project before is dropped.
    let answer: (dto: ReturnType<typeof releasesOf>) => void = () => {};
    vi.mocked(releases).mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    await act(async () => void shown.refreshReleases());
    vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "B" }));
    await act(async () => root.render(<Probe project="/b" />));
    expect(shown.releases?.stem).toBe("B");
    await act(async () => answer(after));
    expect(shown.releases?.stem).toBe("B");
    // No project: nothing.
    await act(async () => root.render(<Probe project={null} />));
    expect(shown.releases).toBeNull();
    expect(shown.releasesError).toBeNull();
  });

  it("no_patch_step_remains", async () => {
    // The old card „Patch“ is gone: releases go through the wizard („Veröffentlichen…“) on the session's build path.
    await act(async () => root.render(<Probe />));
    expect("createPatch" in shown).toBe(false);
    expect("lastPatch" in shown).toBe(false);
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
    expect(shown.errorFrom).toBe("register");
    expect(shown.busy).toBeNull();

    // A register that finds it clears the reason.
    vi.mocked(registerBaseRom).mockResolvedValue(found({ path: "/roms/renamed.nds", source: "known", mark: mark() }));
    await act(async () => shown.register("/roms/renamed.nds"));
    expect(shown.status?.state).toBe("found");
    expect(shown.error).toBeNull();
  });
});
