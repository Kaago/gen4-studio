// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flagUsage } from "../api";
import type { FlagKind, FlagUsageDto, ProjectInfo } from "../api";
import { flagUsageOf } from "./flagTesting";
import { invalidateFlagLists } from "./useFlagList";
import { useFlagUsage } from "./useFlagUsage";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), flagUsage: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useFlagUsage> | null = null;
function Probe({ kind, number, project, revision }: { kind: FlagKind; number: number | null; project: ProjectInfo | null; revision: number }) {
  seen = useFlagUsage(kind, number, { project, revision });
  return null;
}

describe("useFlagUsage", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(flagUsage).mockReset().mockImplementation(async (kind, number) => flagUsageOf(kind, number));
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (kind: FlagKind, number: number | null, p: ProjectInfo | null, revision: number) =>
    act(async () => root.render(<Probe kind={kind} number={number} project={p} revision={revision} />));

  it("reads_on_selection_and_after_a_revision", async () => {
    const p = project();
    // Nothing selected, or no project: nothing is read.
    await show("flag", null, p, 0);
    await show("flag", 0x94, null, 0);
    expect(flagUsage).not.toHaveBeenCalled();
    expect(seen).toEqual({ usage: null, error: null, busy: false });
    // A selection reads its usage; meanwhile it is busy.
    const first = deferred<FlagUsageDto>();
    vi.mocked(flagUsage).mockImplementationOnce(() => first.promise);
    await show("flag", 0x94, p, 0);
    expect(seen).toEqual({ usage: null, error: null, busy: true });
    first.resolve(flagUsageOf("flag", 0x94));
    await flush();
    expect(flagUsage).toHaveBeenLastCalledWith("flag", 0x94);
    expect(seen?.usage?.number).toBe(0x94);
    expect(seen?.busy).toBe(false);
    // The same selection and revision: no second read; a new revision reads again, an invalidation too (an own name).
    await show("flag", 0x94, p, 0);
    await flush();
    expect(flagUsage).toHaveBeenCalledTimes(1);
    await show("flag", 0x94, p, 1);
    await flush();
    expect(flagUsage).toHaveBeenCalledTimes(2);
    await act(async () => invalidateFlagLists());
    await flush();
    expect(flagUsage).toHaveBeenCalledTimes(3);
    // An answer that a newer selection overtook never shows; nor the usage of another number while the new one is read.
    const slow = deferred<FlagUsageDto>();
    vi.mocked(flagUsage).mockImplementationOnce(() => slow.promise);
    await show("var", 0x4000, p, 1);
    expect(seen?.usage).toBeNull();
    await show("flag", 0x173, p, 1);
    await flush();
    slow.resolve(flagUsageOf("var", 0x4000));
    await flush();
    expect(seen?.usage?.number).toBe(0x173);
    // A refusal is the error, with the kernel's sentence.
    vi.mocked(flagUsage).mockRejectedValueOnce({ code: "refused", message: "Nummer 0x0FFF liegt außerhalb des Spielstands.", asset: null });
    await show("flag", 0xfff, p, 1);
    await flush();
    expect(seen).toEqual({ usage: null, error: "Nummer 0x0FFF liegt außerhalb des Spielstands.", busy: false });
  });
});
