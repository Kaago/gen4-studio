import i18next from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import type { BuildStep } from "../api";
import { AGENT_SETTING_UP, agentSettingUpText } from "./TextProjectSurface";
import { stateBadge } from "./diffWords";
import { formatSeconds, patchBuildNote, stepLabel } from "./stepWords";
import { BUILD_CONFLICTS, buildConflictsText, firstSyncNote, syncProgressText } from "./syncWords";
import { changesSubtitle, unbuiltText } from "./ChangesSurface";

const step = (id: string, over: Partial<BuildStep> = {}): BuildStep => ({ id, status: "full", ms: 0, done: 0, total: 0, cached: 0, reason: null, ...over });

afterEach(() => i18next.changeLanguage("de"));

describe("technik in English", () => {
  it("words_the_build_steps_and_durations_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(stepLabel(step("pack:maps", { status: "incremental", done: 1, total: 666 }), "bauen")).toEqual({ name: "Pack maps", result: "1 member inserted" });
    expect(stepLabel(step("pack:maps", { status: "full", done: 666, total: 666 }), "bauen")).toEqual({ name: "Pack maps", result: "666 members packed" });
    expect(stepLabel(step("pack:pokeHeight"), "tuefteln").name).toBe("Pack pokeHeight");
    expect(stepLabel(step("textproject", { reason: "no-state" }), "bauen")).toEqual({ name: "Text project", result: "fully built (first build, full)" });
    expect(stepLabel(step("blz:overlays", { status: "incremental", done: 2, total: 127, cached: 125 }), "bauen").result).toBe("125 from the cache, 2 compressed");
    expect(formatSeconds(330)).toBe("0.33 s");
    expect(formatSeconds(3)).toBe("< 0.01 s");
    expect(patchBuildNote("/p/build/x.nds", "/p")).toBe("Build from “Publish” to build/x.nds");
  });

  it("words_badges_and_subtitles_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(stateBadge("head", "missing")).toEqual({ kind: "error", text: "Deleted" });
    expect(stateBadge("base", "missing")).toEqual({ kind: "error", text: "Missing" });
    expect(stateBadge("saved", "changed")).toEqual({ kind: "unsaved", text: "Unsaved" });
    expect(changesSubtitle("base", null, null, null, null)).toBe("No project open");
    expect(unbuiltText(["a", "b", "c", "d", "e", "f", "g"])).toBe("a, b, c, d, e and 2 more");
  });

  it("words_the_sync_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(syncProgressText({ done: 3, total: 40 })).toBe("Syncing with DSPRE … 3 of 40 files");
    expect(firstSyncNote()).toBe("The first time takes longer; after that it is quick.");
    expect(buildConflictsText()).toBe("Resolve the conflicts in Technical › Text project first.");
  });

  it("keeps_the_German_constants_equal_to_the_catalog_while_other_folders_import_them", () => {
    // `app/useProjectSession.ts` and `app/useTextProjectDialogs.ts` still import these names: until they move to the functions they must
    // say what the German catalog says.
    expect(BUILD_CONFLICTS).toBe(buildConflictsText());
    expect(AGENT_SETTING_UP).toBe(agentSettingUpText());
  });
});
