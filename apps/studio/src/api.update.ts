import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { inTauri, invoke } from "./platform/transport";

/**
 * The updater, mirroring `apps/studio/src-tauri/src/commands/update.rs`. Only inside the packaged app: the commands are the UI's (an
 * agent cannot reach them) and there is no bridge for them, so outside Tauri nothing is asked and `checkForUpdate` finds nothing. Whether
 * the updater runs at all (`!import.meta.env.DEV`) is for the caller to decide. Field names are the Rust ones.
 */

/** A newer version than the installed one: `version` is the new one, `current` the installed one, `notes` the release notes (if any). */
export type UpdateInfo = { version: string; current: string; notes: string | null };

/** What an update under way reports: bytes loaded (`total` null when the size is unknown), then the installation. */
export type UpdateProgress = { phase: "downloading"; done: number; total: number | null } | { phase: "installing" };

/** Looks on GitHub for a newer version: `null` when this is the newest (or outside the app). Rejects with a `CmdError` when it cannot look. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!inTauri()) return null;
  return invoke<UpdateInfo | null>("update_check");
}

/**
 * Downloads `version` (the one `checkForUpdate` found) and keeps the bytes for `installUpdate`; reports `update-progress` while it loads.
 * Nothing is installed yet, so the person can keep working (and editing) meanwhile.
 */
export const downloadUpdate = (version: string) => invoke<void>("update_download", { version });

/**
 * Installs the bytes `downloadUpdate` loaded for `version` and restarts the app. `discardUnsaved` says the person was warned that unsaved
 * changes are lost; without it the backend refuses while the open project has any.
 */
export const installUpdate = (version: string, discardUnsaved: boolean) => invoke<void>("update_install", { version, discardUnsaved });

const nothing = () => {};

/** Listens to the progress of an installation (`update-progress`); resolves to the unsubscribe. Outside the app there is no progress. */
export async function onUpdateProgress(handler: (p: UpdateProgress) => void): Promise<() => void> {
  if (!inTauri()) return nothing;
  return listen<UpdateProgress>("update-progress", (e) => handler(e.payload));
}

/** Listens to the menu entry „Nach Updates suchen…“ (`menu-check-updates`); resolves to the unsubscribe. Outside the app there is no menu. */
export async function onMenuCheckUpdates(handler: () => void): Promise<() => void> {
  if (!inTauri()) return nothing;
  return listen("menu-check-updates", () => handler());
}

/** The installed version („1.0.1“), empty outside the app. */
export async function appVersion(): Promise<string> {
  if (!inTauri()) return "";
  return getVersion();
}
