// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listFlags } from "../api";
import type { FlagKind, FlagListDto, ProjectInfo } from "../api";
import { FLAGS, flagEntry, flagList } from "./flagTesting";
import { invalidateFlagLists, useFlagList } from "./useFlagList";
import type { FlagListDeps } from "./useFlagList";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listFlags: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project = (): ProjectInfo => ({ root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 1 });

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

let seen: ReturnType<typeof useFlagList> | null = null;
function Probe({ kind, project, revision, source, fresh }: { kind: FlagKind } & FlagListDeps) {
  seen = useFlagList(kind, { project, revision, source, fresh });
  return null;
}

describe("useFlagList", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(listFlags).mockReset().mockResolvedValue(FLAGS);
    invalidateFlagLists();
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (kind: FlagKind, p: ProjectInfo | null, revision: number, source?: string, fresh?: boolean) =>
    act(async () => root.render(<Probe kind={kind} project={p} revision={revision} source={source} fresh={fresh} />));

  it("reads_on_open_and_after_a_revision", async () => {
    // No project: nothing to read.
    await show("flag", null, 0);
    expect(listFlags).not.toHaveBeenCalled();
    expect(seen).toMatchObject({ list: null, error: null });
    const p = project();
    await show("flag", p, 0);
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(1);
    expect(listFlags).toHaveBeenLastCalledWith("flag");
    expect(seen?.list).toBe(FLAGS);
    // The same project and revision: no second read.
    await show("flag", p, 0);
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(1);
    // Something changed: read again.
    const later = flagList("flag", [flagEntry(0x94, { own_id: "FLAG_NEU" })]);
    vi.mocked(listFlags).mockResolvedValue(later);
    await show("flag", p, 1);
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(2);
    expect(seen?.list).toBe(later);
    // `reload` reads again without a revision.
    await act(async () => seen?.reload());
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(3);
    // A refusal is shown as its sentence, and no list.
    vi.mocked(listFlags).mockRejectedValue({ code: "error", message: "Index kaputt", asset: null });
    await show("var", p, 1);
    await flush();
    expect(listFlags).toHaveBeenLastCalledWith("var");
    expect(seen).toMatchObject({ list: null, error: "Index kaputt" });
  });

  it("sources_do_not_share_a_read_and_invalidate_reads_again", async () => {
    const p = project();
    const both = () =>
      act(async () =>
        root.render(
          <>
            <Probe kind="var" project={p} revision={3} source="events" />
            <Probe kind="var" project={p} revision={3} source="events" />
          </>,
        ),
      );
    // Two fields of one source and revision: one read.
    await both();
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(1);
    // The same number of another counter is another read.
    await show("var", p, 3, "scripts");
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(2);
    // An own name changed (no revision tells): every list on screen is read again.
    const named = flagList("var", [flagEntry(0x4030, { own_id: "VAR_NEU" })]);
    vi.mocked(listFlags).mockResolvedValue(named);
    await act(async () => invalidateFlagLists());
    await flush();
    expect(listFlags).toHaveBeenCalledTimes(3);
    expect(seen?.list).toBe(named);
  });

  it("fresh_reads_on_every_opening_and_shows_nothing_older", async () => {
    const p = project();
    const later = flagList("flag", [flagEntry(0x96)], { scripts_indexed: true });
    vi.mocked(listFlags).mockResolvedValueOnce({ ...FLAGS, scripts_indexed: false });
    await show("flag", p, 0, "", true);
    await flush();
    expect(seen?.list?.scripts_indexed).toBe(false);
    // Closed (no project) and opened again at the same revision: read anew, the old list gone meanwhile.
    await show("flag", null, 0, "", true);
    const next = deferred<FlagListDto>();
    vi.mocked(listFlags).mockReturnValueOnce(next.promise);
    await show("flag", p, 0, "", true);
    expect(listFlags).toHaveBeenCalledTimes(2);
    expect(seen?.list).toBeNull();
    await act(async () => next.resolve(later));
    await flush();
    expect(seen?.list).toBe(later);
  });

  it("drops_an_older_answer", async () => {
    const slow = deferred<FlagListDto>();
    const fast = flagList("flag", [flagEntry(0x96)]);
    vi.mocked(listFlags).mockReturnValueOnce(slow.promise).mockResolvedValueOnce(fast);
    const p = project();
    await show("flag", p, 0);
    await show("flag", p, 1);
    await flush();
    expect(seen?.list).toBe(fast);
    // The answer for revision 0 comes late: it never shows.
    await act(async () => slow.resolve(FLAGS));
    await flush();
    expect(seen?.list).toBe(fast);
    // Another project never shows the list of the one before.
    const other = project();
    vi.mocked(listFlags).mockReturnValueOnce(new Promise(() => {}));
    await show("flag", other, 1);
    expect(seen?.list).toBeNull();
  });
});
