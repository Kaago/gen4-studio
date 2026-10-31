import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const inTauri = vi.hoisted(() => vi.fn(() => true));
vi.mock("./platform/transport", () => ({ invoke, inTauri }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    onmessage: (m: unknown) => void = () => {};
  },
}));

import { gitCommitAll, isCmdError, publishPatch, releases, revealPath } from "./api";

/** The arguments of the last call. */
const last = () => invoke.mock.calls[invoke.mock.calls.length - 1]!;

describe("publish commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    inTauri.mockReturnValue(true);
  });

  it("publish_commands_send_the_documented_arguments", async () => {
    await releases();
    expect(invoke).toHaveBeenLastCalledWith("releases");

    // A release: the options flat (as `dispatch`, the Tauri command and the MCP tool take them), the phases over a channel.
    const onPublish = vi.fn();
    await publishPatch({ version: "0.1.0", notes: "Neue Karte", dir: "/h/release", file: "Hack.xdelta", readme: true, overwrite: false }, onPublish);
    let [cmd, args] = last();
    expect(cmd).toBe("publish_patch");
    expect(args).toMatchObject({ version: "0.1.0", notes: "Neue Karte", dir: "/h/release", file: "Hack.xdelta", readme: true, overwrite: false });
    args.progress.onmessage({ phase: "verify" });
    expect(onPublish).toHaveBeenCalledWith({ phase: "verify" });
    // Only the version is needed: the rest is left out, the backend takes its defaults.
    await publishPatch({ version: "0.2.0" }, onPublish);
    expect(Object.keys(last()[1]).sort()).toEqual(["progress", "version"]);

    await revealPath("/h/release/Hack.xdelta");
    expect(invoke).toHaveBeenLastCalledWith("reveal_path", { path: "/h/release/Hack.xdelta" });

    // „Stand sichern“: subject and note flat, the phases over a channel.
    const onCommit = vi.fn();
    await gitCommitAll({ message: "Veröffentlicht v0.1.0", notes: "Neue Karte" }, onCommit);
    [cmd, args] = last();
    expect(cmd).toBe("git_commit_all");
    expect(args).toMatchObject({ message: "Veröffentlicht v0.1.0", notes: "Neue Karte" });
    args.progress.onmessage({ phase: "add" });
    expect(onCommit).toHaveBeenCalledWith({ phase: "add" });

    // The dev bridge has no channel: the calls run without one and report no progress.
    inTauri.mockReturnValue(false);
    invoke.mockReset();
    invoke.mockResolvedValue(null);
    await publishPatch({ version: "0.1.0", overwrite: true }, onPublish);
    await gitCommitAll({}, onCommit);
    expect(invoke.mock.calls).toEqual([
      ["publish_patch", { version: "0.1.0", overwrite: true }],
      ["git_commit_all", {}],
    ]);
  });

  it("publish_refusals_are_command_errors", () => {
    for (const code of ["unsaved", "base_missing", "bad_version", "version_taken", "bad_file", "bad_dir", "exists", "no_changes", "releases_broken", "busy", "no_repo", "enclosing_repo", "nothing_to_commit", "no_git"]) {
      expect(isCmdError({ code, message: "Satz.", asset: null })).toBe(true);
    }
  });
});
