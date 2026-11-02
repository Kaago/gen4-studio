import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { listThreads, storyCreate, storyGet, storyList, storyUpdate, threadGraph } from "./api";

describe("story commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("story_commands_send_the_documented_arguments", async () => {
    await listThreads();
    expect(invoke).toHaveBeenLastCalledWith("list_threads", {});
    // A thread alone sends no story; with one, its id.
    await threadGraph(0x4031);
    expect(invoke).toHaveBeenLastCalledWith("thread_graph", { var: 0x4031, story: null });
    await threadGraph(0x4031, "STORY_RIVALIN");
    expect(invoke).toHaveBeenLastCalledWith("thread_graph", { var: 0x4031, story: "STORY_RIVALIN" });
    await storyList();
    expect(invoke).toHaveBeenLastCalledWith("story_list", {});
    await storyGet("STORY_RIVALIN");
    expect(invoke).toHaveBeenLastCalledWith("story_get", { id: "STORY_RIVALIN" });
    await storyCreate({ name: "Rivalin", progress: { var: "VAR_RIVALIN" }, notes: "Am See." });
    expect(invoke).toHaveBeenLastCalledWith("story_create", { req: { name: "Rivalin", progress: { var: "VAR_RIVALIN" }, notes: "Am See." } });
    await storyUpdate("STORY_RIVALIN", { notes: "Am See, dann im Labor.", layout: { nodes: { s1: { x: 4, y: 8 } } } });
    expect(invoke).toHaveBeenLastCalledWith("story_update", { id: "STORY_RIVALIN", patch: { notes: "Am See, dann im Labor.", layout: { nodes: { s1: { x: 4, y: 8 } } } } });
  });
});
