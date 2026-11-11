import { useCallback, useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { useStableCallback } from "../app/useStable";
import { errorText } from "../api";
import { formatBytes } from "../api.base";
import { appVersion, checkForUpdate, downloadUpdate, installUpdate, onMenuCheckUpdates, onUpdateProgress } from "../api.update";
import type { UpdateInfo, UpdateProgress } from "../api.update";
import { t } from "../i18n";
import { ask } from "../platform/dialogs";
import { inTauri } from "../platform/transport";
import { toast } from "../ui/frame/toasts";
import { isCheckDue, readUpdatePrefs, writeUpdatePrefs } from "./updatePrefs";
import type { UpdatePrefs } from "./updatePrefs";

/** What `useUpdates` uses from the outside; every field can be replaced (tests). `storage` is `localStorage` by default. */
export type UpdateDeps = {
  /** The updater only runs in the packaged app. */
  enabled: boolean;
  check: typeof checkForUpdate;
  download: typeof downloadUpdate;
  install: typeof installUpdate;
  onProgress: typeof onUpdateProgress;
  onMenu: typeof onMenuCheckUpdates;
  version: typeof appVersion;
  ask: typeof ask;
  now: () => number;
  storage: Storage;
};

/** What the Settings and the frame read and do. */
export type UpdatesControl = {
  /** False in the dev build, in the browser mode and in tests: nothing below runs and the Settings show no column. */
  enabled: boolean;
  prefs: UpdatePrefs;
  /** The installed version („1.0.1“); empty until it is known. */
  version: string;
  /** A check is running. */
  checking: boolean;
  /** The switch in the Settings: `true` means consent „auto“, `false` „off“. No check on its own. */
  setAuto(on: boolean): void;
  /** „Jetzt suchen“ and the menu entry: looks now and says what it found, or why it could not. */
  checkNow(): void;
};

/** The toast of the offer and, kept apart by its kind, of the progress. */
const TOAST_KEY = "update";
/** The notes in the question are cut to this many characters. */
const NOTES_MAX = 1500;

/** The question before an installation: version, a warning when unsaved changes are lost, and the notes (cut when long). */
function confirmText(info: UpdateInfo, unsaved: boolean): string {
  let text = t("app:updates.confirm", { version: info.version, current: info.current });
  if (unsaved) text += `\n\n${t("app:updates.unsavedWarning")}`;
  const notes = info.notes?.trim();
  if (notes) {
    // By characters, not UTF-16 units: a cut must not split a symbol in two.
    const chars = Array.from(notes);
    text += `\n\n${t("app:updates.notesHeading")}\n${chars.length > NOTES_MAX ? `${chars.slice(0, NOTES_MAX).join("")}…` : notes}`;
  }
  return text;
}

/** „Update wird geladen… 50 %“, or with the bytes when the size is unknown. */
function downloadingText(done: number, total: number | null): string {
  if (total === null || total <= 0) return t("app:updates.downloadingBytes", { size: formatBytes(done) });
  return t("app:updates.downloading", { percent: Math.min(100, Math.floor((done * 100) / total)) });
}

/**
 * The whole update flow of the app, for as long as it is mounted: asks once whether to look for updates, looks at startup when due (at
 * most once a day, silently), offers a new version in a toast, asks before installing, shows the progress and the errors. `unsavedRef` is
 * the number of unsaved edits (read when asked): they are mentioned in the question and, when they appear while the update downloads,
 * asked about again before it is installed. Nothing runs when `enabled` is false.
 */
export function useUpdates(unsavedRef: MutableRefObject<number>, deps?: Partial<UpdateDeps>): UpdatesControl {
  const used: Omit<UpdateDeps, "storage"> & { storage?: Storage } = {
    enabled: inTauri() && !import.meta.env.DEV,
    check: checkForUpdate,
    download: downloadUpdate,
    install: installUpdate,
    onProgress: onUpdateProgress,
    onMenu: onMenuCheckUpdates,
    version: appVersion,
    ask,
    now: Date.now,
    // `localStorage` is not touched here (a blocked one throws on access): `updatePrefs` falls back to it and catches that.
    ...deps,
  };
  // The callbacks below read the latest values through this ref, so none of them changes identity.
  const latest = useRef(used);
  latest.current = used;
  const { enabled } = used;

  const [prefs, setPrefs] = useState(() => readUpdatePrefs(used.storage));
  // What is remembered, in memory as well: a storage that refuses to save is still what this start goes by.
  const prefsRef = useRef(prefs);
  const [version, setVersion] = useState("");
  const versionLoad = useRef<Promise<string>>(Promise.resolve(""));
  const [checking, setChecking] = useState(false);
  const checkRunning = useRef(false);
  // From the question until the installation is done (it stays set after a success: the app restarts).
  const installRunning = useRef(false);
  const started = useRef(false);

  const remember = useCallback((patch: Partial<UpdatePrefs>) => {
    const next = { ...prefsRef.current, ...patch };
    prefsRef.current = next;
    setPrefs(next);
    writeUpdatePrefs(next, latest.current.storage);
  }, []);

  const skip = useStableCallback((info: UpdateInfo) => {
    remember({ skipped: info.version });
    toast.dismissKey(TOAST_KEY);
  });

  const confirm = (info: UpdateInfo, unsaved: boolean) =>
    latest.current.ask(confirmText(info, unsaved), {
      kind: unsaved ? "warning" : "info",
      title: t("app:updates.confirmTitle"),
      okLabel: t("app:updates.installAndRestart"),
      cancelLabel: t("app:updates.cancel"),
    });

  /** „Install…“: ask, download (edits made meanwhile are asked about again), install. One at a time. */
  const startInstall = useStableCallback(async (info: UpdateInfo) => {
    if (installRunning.current) return;
    installRunning.current = true;
    let installed = false;
    // Where a failure happened: the backend sends only the cause, the toast names the stage once.
    let stage: "download" | "install" = "install";
    try {
      const unsavedAtConfirm = unsavedRef.current;
      if (!(await confirm(info, unsavedAtConfirm > 0))) return;
      toast.dismissKey(TOAST_KEY);
      toast.progress(TOAST_KEY, t("app:updates.downloading", { percent: 0 }));
      stage = "download";
      await latest.current.download(info.version);
      stage = "install";
      if (unsavedRef.current > unsavedAtConfirm && !(await confirm(info, true))) {
        toast.progress(TOAST_KEY, null);
        return;
      }
      await latest.current.install(info.version, unsavedRef.current > 0);
      installed = true;
    } catch (e) {
      toast.progress(TOAST_KEY, null);
      const reason = errorText(e);
      toast.error(stage === "download" ? t("app:updates.downloadFailed", { reason }) : t("app:updates.installFailed", { reason }));
    } finally {
      if (!installed) installRunning.current = false;
    }
  });

  const offer = useStableCallback((info: UpdateInfo) => {
    toast.lastingInfo(t("app:updates.available", { version: info.version }), {
      key: TOAST_KEY,
      actions: [
        { label: t("app:updates.install"), onClick: () => void startInstall(info) },
        { label: t("app:updates.skip"), onClick: () => skip(info) },
      ],
    });
  });

  /** One check at a time, and none during an installation. `manual`: it says what it found, or why it failed; else it stays silent. */
  const runCheck = useStableCallback(async (manual: boolean) => {
    if (!latest.current.enabled || checkRunning.current || installRunning.current) return;
    checkRunning.current = true;
    setChecking(true);
    try {
      let found: UpdateInfo | null;
      try {
        found = await latest.current.check();
      } catch (e) {
        // lastCheck stays: the next start tries again.
        if (manual) toast.error(t("app:updates.checkFailed", { reason: errorText(e) }));
        return;
      }
      remember({ lastCheck: new Date(latest.current.now()).toISOString() });
      if (found === null) {
        if (manual) toast.info(t("app:updates.upToDate", { version: await versionLoad.current }));
      } else if (manual || prefsRef.current.skipped !== found.version) {
        offer(found);
      }
    } finally {
      checkRunning.current = false;
      setChecking(false);
    }
  });

  /** At startup: the question once while the consent is open, then the check when it is due. */
  const startup = useStableCallback(async () => {
    if (prefsRef.current.consent === "ask") {
      let yes: boolean;
      try {
        yes = await latest.current.ask(t("app:updates.consent"), {
          kind: "info",
          title: t("app:updates.consentTitle"),
          okLabel: t("app:updates.consentYes"),
          cancelLabel: t("app:updates.consentNo"),
        });
      } catch {
        return;
      }
      remember({ consent: yes ? "auto" : "off" });
    }
    if (isCheckDue(prefsRef.current, latest.current.now())) await runCheck(false);
  });

  const progress = useStableCallback((p: UpdateProgress) => {
    toast.progress(TOAST_KEY, p.phase === "installing" ? t("app:updates.installing") : downloadingText(p.done, p.total));
  });

  // Once per mount (the ref also holds against the second run of an effect in StrictMode).
  useEffect(() => {
    if (!enabled || started.current) return;
    started.current = true;
    void startup();
  }, [enabled, startup]);

  useEffect(() => {
    if (!enabled) return;
    let current = true;
    versionLoad.current = (async () => {
      try {
        return await latest.current.version();
      } catch {
        return "";
      }
    })();
    void versionLoad.current.then((v) => {
      if (current) setVersion(v);
    });
    return () => {
      current = false;
    };
  }, [enabled]);

  // The progress of an installation and the menu entry.
  useEffect(() => {
    if (!enabled) return;
    let disposed = false;
    const unlisten: (() => void)[] = [];
    const keep = (listening: Promise<() => void>) =>
      listening
        .then((stop) => {
          if (disposed) stop();
          else unlisten.push(stop);
        })
        .catch(() => {
          /* not running inside a Tauri window */
        });
    void keep(latest.current.onProgress(progress));
    void keep(latest.current.onMenu(() => void runCheck(true)));
    return () => {
      disposed = true;
      for (const stop of unlisten) stop();
    };
  }, [enabled, progress, runCheck]);

  const setAuto = useCallback((on: boolean) => remember({ consent: on ? "auto" : "off" }), [remember]);
  const checkNow = useCallback(() => void runCheck(true), [runCheck]);
  return { enabled, prefs, version, checking, setAuto, checkNow };
}
