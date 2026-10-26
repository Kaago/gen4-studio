// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PreviewDto } from "../api";
import { StartServiceContext } from "./startService";
import type { StartService } from "./startService";
import { usePreviews } from "./usePreviews";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const dto = (png: string): PreviewDto => ({ source: "eigen", png, standardMap: null });

describe("usePreviews", () => {
  let container: HTMLElement;
  let root: Root;
  let result: ReturnType<typeof usePreviews>;
  const get = vi.fn<(path?: string) => Promise<PreviewDto>>();
  const service = { projectPreviewGet: get } as unknown as StartService;

  function Probe({ paths, missing, visible }: { paths: readonly string[]; missing: ReadonlySet<string>; visible: boolean }) {
    result = usePreviews(paths, missing, visible);
    return null;
  }
  const render = async (paths: readonly string[], visible = true, missing: ReadonlySet<string> = new Set()) =>
    act(async () =>
      root.render(
        <StartServiceContext.Provider value={service}>
          <Probe paths={paths} missing={missing} visible={visible} />
        </StartServiceContext.Provider>,
      ),
    );

  beforeEach(() => {
    get.mockReset();
    get.mockImplementation(async (path) => dto(`png-of-${path}`));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("loads each path once", async () => {
    await render(["/a", "/b"]);
    expect(get.mock.calls.map((c) => c[0]).sort()).toEqual(["/a", "/b"]);
    expect(result.previews.get("/a")?.png).toBe("png-of-/a");
    expect(result.previews.get("/b")?.png).toBe("png-of-/b");
    // Rendering again with the same paths (a new array) loads nothing; a new path is loaded alone.
    await render(["/a", "/b"]);
    await render(["/c", "/a", "/b"]);
    expect(get.mock.calls.map((c) => c[0]).sort()).toEqual(["/a", "/b", "/c"]);
  });

  it("skips the missing ones", async () => {
    await render(["/a", "/gone"], true, new Set(["/gone"]));
    expect(get.mock.calls.map((c) => c[0])).toEqual(["/a"]);
    expect(result.previews.has("/gone")).toBe(false);
    // Found again (it opened): loaded then.
    await render(["/a", "/gone"], true, new Set());
    expect(get.mock.calls.map((c) => c[0])).toEqual(["/a", "/gone"]);
  });

  it("loads again when it becomes visible", async () => {
    await render(["/a"], false);
    expect(get).not.toHaveBeenCalled();
    await render(["/a"], true);
    expect(get).toHaveBeenCalledTimes(1);
    await render(["/a"], false);
    get.mockImplementation(async () => dto("new"));
    await render(["/a"], true);
    expect(get).toHaveBeenCalledTimes(2);
    expect(result.previews.get("/a")?.png).toBe("new");
  });

  it("an error gives no entry", async () => {
    get.mockImplementation(async (path) => {
      if (path === "/bad") throw { code: "error", message: "boom", asset: null };
      if (path === "/odd") return undefined as unknown as PreviewDto;
      return dto("ok");
    });
    await render(["/bad", "/odd", "/a"]);
    expect(result.previews.has("/bad")).toBe(false);
    expect(result.previews.has("/odd")).toBe(false);
    expect(result.previews.get("/a")?.png).toBe("ok");
  });

  it("an answer without a picture gives no entry, and reload drops a removed picture", async () => {
    await render(["/a"]);
    expect(result.previews.has("/a")).toBe(true);
    get.mockImplementation(async () => ({ source: null, png: null, standardMap: null }));
    await act(async () => result.reload("/a"));
    expect(get).toHaveBeenCalledTimes(2);
    expect(result.previews.has("/a")).toBe(false);
  });
});
