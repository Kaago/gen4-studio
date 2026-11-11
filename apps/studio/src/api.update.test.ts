import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const tauri = vi.hoisted(() => ({ on: true }));
const listen = vi.hoisted(() => vi.fn());
const getVersion = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => tauri.on }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion }));

import { appVersion, checkForUpdate, installUpdate, onMenuCheckUpdates, onUpdateProgress } from "./api.update";
import type { UpdateProgress } from "./api.update";

describe("update commands outside Tauri", () => {
  beforeEach(() => {
    tauri.on = false;
    invoke.mockReset();
    listen.mockReset();
    getVersion.mockReset();
  });

  it("check_for_update_is_null_without_calling_the_backend", async () => {
    await expect(checkForUpdate()).resolves.toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("app_version_is_empty", async () => {
    await expect(appVersion()).resolves.toBe("");
    expect(getVersion).not.toHaveBeenCalled();
  });

  it("listening_gives_an_unsubscribe_that_does_nothing", async () => {
    const stopProgress = await onUpdateProgress(() => {});
    const stopMenu = await onMenuCheckUpdates(() => {});
    expect(listen).not.toHaveBeenCalled();
    expect(() => {
      stopProgress();
      stopMenu();
    }).not.toThrow();
  });
});

describe("update commands inside Tauri", () => {
  beforeEach(() => {
    tauri.on = true;
    invoke.mockReset();
    listen.mockReset();
    getVersion.mockReset();
  });

  it("check_for_update_calls_update_check_and_passes_the_answer_on", async () => {
    invoke.mockResolvedValueOnce({ version: "1.0.2", current: "1.0.1", notes: "Neu: Updates" });
    await expect(checkForUpdate()).resolves.toEqual({ version: "1.0.2", current: "1.0.1", notes: "Neu: Updates" });
    expect(invoke).toHaveBeenCalledWith("update_check");
    invoke.mockResolvedValueOnce(null);
    await expect(checkForUpdate()).resolves.toBeNull();
  });

  it("check_for_update_rejects_with_the_error_of_the_backend", async () => {
    const error = { code: "error", message: "offline", asset: null };
    invoke.mockRejectedValueOnce(error);
    await expect(checkForUpdate()).rejects.toBe(error);
  });

  it("install_update_sends_the_version_and_the_discard_flag", async () => {
    invoke.mockResolvedValue(undefined);
    await installUpdate("1.0.2", true);
    expect(invoke).toHaveBeenLastCalledWith("update_install", { version: "1.0.2", discardUnsaved: true });
    await installUpdate("1.0.3", false);
    expect(invoke).toHaveBeenLastCalledWith("update_install", { version: "1.0.3", discardUnsaved: false });
  });

  it("on_update_progress_listens_to_update_progress_and_passes_the_payload", async () => {
    const unlisten = vi.fn();
    listen.mockResolvedValue(unlisten);
    const handler = vi.fn();
    const stop = await onUpdateProgress(handler);
    expect(listen).toHaveBeenCalledTimes(1);
    expect(listen.mock.calls[0]![0]).toBe("update-progress");
    expect(stop).toBe(unlisten);
    const emit = listen.mock.calls[0]![1] as (e: { payload: UpdateProgress }) => void;
    emit({ payload: { phase: "downloading", done: 10, total: 100 } });
    emit({ payload: { phase: "installing" } });
    expect(handler.mock.calls).toEqual([[{ phase: "downloading", done: 10, total: 100 }], [{ phase: "installing" }]]);
  });

  it("on_menu_check_updates_listens_to_menu_check_updates_and_calls_without_the_payload", async () => {
    const unlisten = vi.fn();
    listen.mockResolvedValue(unlisten);
    const handler = vi.fn();
    const stop = await onMenuCheckUpdates(handler);
    expect(listen.mock.calls[0]![0]).toBe("menu-check-updates");
    expect(stop).toBe(unlisten);
    (listen.mock.calls[0]![1] as (e: { payload: unknown }) => void)({ payload: null });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith();
  });

  it("app_version_reads_the_version_of_the_app", async () => {
    getVersion.mockResolvedValue("1.0.1");
    await expect(appVersion()).resolves.toBe("1.0.1");
  });
});
