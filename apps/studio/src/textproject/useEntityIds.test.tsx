// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { entityIds } from "../api";
import type { EntityKind } from "../api";
import { EntityIdCache, TextProjectProvider } from "./TextProjectContext";
import { useEntityIds } from "./useEntityIds";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  entityIds: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

function Probe({ kind, enabled, index }: { kind: EntityKind; enabled: boolean; index: number }) {
  const ids = useEntityIds(kind, enabled);
  return <span data-probe={kind}>{ids === null ? "–" : (ids[index] ?? "?")}</span>;
}

describe("useEntityIds", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(() => {
    vi.mocked(entityIds)
      .mockReset()
      .mockImplementation(async (kind): Promise<Record<number, string>> => (kind === "Trainer" ? { 1: "TRAINER_JENS" } : { 25: "SPECIES_PIKACHU" }));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const texts = () => [...container.querySelectorAll("[data-probe]")].map((p) => p.textContent);

  it("reads_once_per_kind_and_only_when_enabled", async () => {
    const cache = new EntityIdCache();
    const show = (enabled: boolean, extra: boolean) =>
      act(async () =>
        root.render(
          <TextProjectProvider active ids={cache} onRenamed={() => {}}>
            <Probe kind="Trainer" enabled={enabled} index={1} />
            <Probe kind="Trainer" enabled={enabled} index={1} />
            {extra && <Probe kind="Species" enabled={enabled} index={25} />}
          </TextProjectProvider>,
        ),
      );
    // Not enabled (a DSPRE folder): nothing is read, nothing shown.
    await show(false, true);
    await flush();
    expect(entityIds).not.toHaveBeenCalled();
    expect(texts()).toEqual(["–", "–", "–"]);
    // Two heads of one kind share one read.
    await show(true, false);
    await flush();
    expect(entityIds).toHaveBeenCalledTimes(1);
    expect(entityIds).toHaveBeenCalledWith("Trainer");
    expect(texts()).toEqual(["TRAINER_JENS", "TRAINER_JENS"]);
    // Another kind is read once more; the first is not read again.
    await show(true, true);
    await flush();
    expect(entityIds).toHaveBeenCalledTimes(2);
    expect(entityIds).toHaveBeenLastCalledWith("Species");
    expect(texts()).toEqual(["TRAINER_JENS", "TRAINER_JENS", "SPECIES_PIKACHU"]);
    // A new cache (another session, a rename, an undo) reads again.
    const fresh = new EntityIdCache();
    await act(async () =>
      root.render(
        <TextProjectProvider active ids={fresh} onRenamed={() => {}}>
          <Probe kind="Trainer" enabled index={1} />
        </TextProjectProvider>,
      ),
    );
    await flush();
    expect(entityIds).toHaveBeenCalledTimes(3);
  });
});
