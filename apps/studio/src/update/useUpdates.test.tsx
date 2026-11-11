// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { formatBytes } from "../api.base";
import type { UpdateInfo, UpdateProgress } from "../api.update";
import { t } from "../i18n";
import { listToasts, resetToasts } from "../ui/frame/toasts";
import { CHECK_INTERVAL_MS, UPDATE_PREFS_KEY } from "./updatePrefs";
import type { UpdatePrefs } from "./updatePrefs";
import { useUpdates } from "./useUpdates";
import type { UpdateDeps, UpdatesControl } from "./useUpdates";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const NOW = Date.parse("2026-11-12T12:00:00Z");
const NEW: UpdateInfo = { version: "1.0.2", current: "1.0.1", notes: "- Fix" };

/** A `Storage` kept in a map; `setItem` may be made to throw. */
function storage(prefs?: Partial<UpdatePrefs>, opts: { setThrows?: boolean } = {}): Storage & { map: Map<string, string> } {
  const map = new Map<string, string>();
  if (prefs) map.set(UPDATE_PREFS_KEY, JSON.stringify(prefs));
  return {
    map,
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (k: string) => map.get(k) ?? null,
    key: (i: number) => [...map.keys()][i] ?? null,
    removeItem: (k: string) => void map.delete(k),
    setItem: (k: string, v: string) => {
      if (opts.setThrows) throw new Error("blocked");
      map.set(k, v);
    },
  };
}

const saved = (s: ReturnType<typeof storage>): UpdatePrefs => JSON.parse(s.map.get(UPDATE_PREFS_KEY) ?? "{}") as UpdatePrefs;
const auto = (over: Partial<UpdatePrefs> = {}): Partial<UpdatePrefs> => ({ consent: "auto", lastCheck: null, skipped: null, ...over });
const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
/** A promise that is settled from outside. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("useUpdates", () => {
  let container: HTMLElement;
  let root: Root;
  let control: UpdatesControl;
  let emitProgress: (p: UpdateProgress) => void;
  let emitMenu: () => void;
  const unlistenProgress = vi.fn();
  const unlistenMenu = vi.fn();
  let unsaved: { current: number };

  const makeDeps = (over: Partial<UpdateDeps> = {}): UpdateDeps => ({
    enabled: true,
    check: vi.fn(async () => null),
    download: vi.fn(async () => {}),
    install: vi.fn(async () => {}),
    onProgress: vi.fn(async (handler: (p: UpdateProgress) => void) => {
      emitProgress = handler;
      return unlistenProgress;
    }),
    onMenu: vi.fn(async (handler: () => void) => {
      emitMenu = handler;
      return unlistenMenu;
    }),
    version: vi.fn(async () => "1.0.1"),
    ask: vi.fn(async () => true),
    now: () => NOW,
    storage: storage(auto({ lastCheck: new Date(NOW - 2 * CHECK_INTERVAL_MS).toISOString() })),
    ...over,
  });

  function Probe({ deps }: { deps: UpdateDeps }) {
    control = useUpdates(unsaved, deps);
    return null;
  }
  const mount = async (deps: UpdateDeps) => {
    await act(async () => root.render(<Probe deps={deps} />));
    await flush();
  };
  /** The toast of the given kind (the offer is an info, the progress a progress; both have the key `update`). */
  const toastOf = (kind: "info" | "progress" = "info") => listToasts().find((x) => x.key === "update" && x.kind === kind);
  const press = async (label: string) => {
    const action = toastOf()?.actions.find((a) => a.label === label);
    if (!action) throw new Error(`no action ${label}`);
    await act(async () => action.onClick());
    await flush();
  };

  beforeEach(() => {
    resetToasts();
    unlistenProgress.mockClear();
    unlistenMenu.mockClear();
    unsaved = { current: 0 };
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    resetToasts();
  });

  it("does_nothing_when_not_enabled", async () => {
    const deps = makeDeps({ enabled: false, storage: storage({ consent: "ask" }) });
    await mount(deps);
    expect(control.enabled).toBe(false);
    expect(deps.ask).not.toHaveBeenCalled();
    expect(deps.check).not.toHaveBeenCalled();
    expect(deps.onProgress).not.toHaveBeenCalled();
    expect(deps.onMenu).not.toHaveBeenCalled();
    act(() => control.checkNow());
    expect(deps.check).not.toHaveBeenCalled();
  });

  describe("consent", () => {
    it("asks_once_with_the_labeled_question_and_a_yes_means_auto_and_a_check", async () => {
      const store = storage({ consent: "ask" });
      const deps = makeDeps({ storage: store });
      await mount(deps);
      expect(deps.ask).toHaveBeenCalledTimes(1);
      expect(deps.ask).toHaveBeenCalledWith(t("app:updates.consent"), { kind: "info", title: "Updates", okLabel: "Automatisch suchen", cancelLabel: "Nicht suchen" });
      expect(saved(store).consent).toBe("auto");
      expect(control.prefs.consent).toBe("auto");
      expect(deps.check).toHaveBeenCalledTimes(1);
    });

    it("a_no_means_off_and_no_check", async () => {
      const store = storage({ consent: "ask" });
      const deps = makeDeps({ storage: store, ask: vi.fn(async () => false) });
      await mount(deps);
      expect(saved(store).consent).toBe("off");
      expect(control.prefs.consent).toBe("off");
      expect(deps.check).not.toHaveBeenCalled();
    });

    it("asks_only_once_even_when_the_storage_cannot_be_written", async () => {
      const store = storage({ consent: "ask" }, { setThrows: true });
      const deps = makeDeps({ storage: store });
      await mount(deps);
      await mount(deps);
      await mount(makeDeps({ storage: store, ask: deps.ask }));
      expect(deps.ask).toHaveBeenCalledTimes(1);
      expect(control.prefs.consent).toBe("auto");
    });

    it("a_question_that_fails_leaves_the_consent_alone_and_checks_nothing", async () => {
      const store = storage({ consent: "ask" });
      const deps = makeDeps({ storage: store, ask: vi.fn(async () => Promise.reject(new Error("no dialog"))) });
      await mount(deps);
      expect(control.prefs.consent).toBe("ask");
      expect(deps.check).not.toHaveBeenCalled();
    });

    it("setAuto_sets_auto_or_off_without_a_check", async () => {
      const store = storage({ consent: "off" });
      const deps = makeDeps({ storage: store });
      await mount(deps);
      act(() => control.setAuto(true));
      expect(saved(store).consent).toBe("auto");
      expect(control.prefs.consent).toBe("auto");
      act(() => control.setAuto(false));
      expect(saved(store).consent).toBe("off");
      expect(control.prefs.consent).toBe("off");
      expect(deps.check).not.toHaveBeenCalled();
    });
  });

  describe("the check at startup", () => {
    it("finding_nothing_is_silent_and_sets_lastCheck_to_now", async () => {
      const store = storage(auto({ lastCheck: new Date(NOW - 2 * CHECK_INTERVAL_MS).toISOString() }));
      const deps = makeDeps({ storage: store });
      await mount(deps);
      expect(deps.check).toHaveBeenCalledTimes(1);
      expect(listToasts()).toEqual([]);
      expect(saved(store).lastCheck).toBe(new Date(NOW).toISOString());
      expect(control.prefs.lastCheck).toBe(new Date(NOW).toISOString());
    });

    it("failing_is_silent_and_keeps_lastCheck_so_the_next_start_tries_again", async () => {
      const before = new Date(NOW - 2 * CHECK_INTERVAL_MS).toISOString();
      const store = storage(auto({ lastCheck: before }));
      const deps = makeDeps({ storage: store, check: vi.fn(async () => Promise.reject(new Error("offline"))) });
      await mount(deps);
      expect(deps.check).toHaveBeenCalledTimes(1);
      expect(listToasts()).toEqual([]);
      expect(saved(store).lastCheck).toBe(before);
      expect(control.prefs.lastCheck).toBe(before);
      expect(control.checking).toBe(false);
    });

    it("does_not_check_when_it_is_not_due", async () => {
      const deps = makeDeps({ storage: storage(auto({ lastCheck: new Date(NOW - 3_600_000).toISOString() })) });
      await mount(deps);
      expect(deps.check).not.toHaveBeenCalled();
    });

    it("does_not_check_with_consent_off", async () => {
      const deps = makeDeps({ storage: storage({ consent: "off" }) });
      await mount(deps);
      expect(deps.check).not.toHaveBeenCalled();
      expect(deps.ask).not.toHaveBeenCalled();
    });

    it("checks_once_per_mount_however_often_it_renders", async () => {
      const deps = makeDeps();
      await mount(deps);
      await mount(deps);
      expect(deps.check).toHaveBeenCalledTimes(1);
    });
  });

  describe("an update is found", () => {
    const found = (over: Partial<UpdateDeps> = {}) => makeDeps({ check: vi.fn(async () => NEW), ...over });

    it("shows_a_lasting_info_with_install_and_skip", async () => {
      await mount(found());
      const toast = toastOf();
      expect(listToasts()).toHaveLength(1);
      expect(toast?.kind).toBe("info");
      expect(toast?.text).toBe("Gen4 Studio 1.0.2 ist verfügbar.");
      expect(toast?.actions.map((a) => a.label)).toEqual(["Installieren…", "Diese Version überspringen"]);
    });

    it("the_automatic_check_does_not_report_a_skipped_version_but_checkNow_does", async () => {
      const deps = found({ storage: storage(auto({ lastCheck: null, skipped: "1.0.2" })) });
      await mount(deps);
      expect(deps.check).toHaveBeenCalledTimes(1);
      expect(listToasts()).toEqual([]);
      await act(async () => control.checkNow());
      await flush();
      expect(deps.check).toHaveBeenCalledTimes(2);
      expect(toastOf()?.text).toBe("Gen4 Studio 1.0.2 ist verfügbar.");
    });

    it("skipping_remembers_the_version_and_closes_the_toast", async () => {
      const store = storage(auto());
      await mount(found({ storage: store }));
      await press("Diese Version überspringen");
      expect(saved(store).skipped).toBe("1.0.2");
      expect(control.prefs.skipped).toBe("1.0.2");
      expect(toastOf()).toBeUndefined();
    });
  });

  describe("checkNow", () => {
    const quiet = (over: Partial<UpdateDeps> = {}) => makeDeps({ storage: storage({ consent: "off" }), ...over });

    it("says_it_is_up_to_date_with_the_installed_version", async () => {
      const deps = quiet();
      await mount(deps);
      await act(async () => control.checkNow());
      await flush();
      expect(listToasts().map((x) => [x.kind, x.text])).toEqual([["info", "Gen4 Studio 1.0.1 ist aktuell."]]);
      expect(control.version).toBe("1.0.1");
    });

    it("shows_an_error_toast_when_it_fails", async () => {
      const deps = quiet({ check: vi.fn(async () => Promise.reject({ code: "error", message: "kein Netz", asset: null })) });
      await mount(deps);
      await act(async () => control.checkNow());
      await flush();
      expect(listToasts().map((x) => [x.kind, x.text])).toEqual([["error", "Die Suche nach Updates ist gescheitert: kein Netz"]]);
      expect(control.prefs.lastCheck).toBeNull();
    });

    it("a_second_call_during_a_running_check_does_not_check_again", async () => {
      const pending = deferred<UpdateInfo | null>();
      const deps = quiet({ check: vi.fn(() => pending.promise) });
      await mount(deps);
      await act(async () => {
        control.checkNow();
        control.checkNow();
      });
      expect(deps.check).toHaveBeenCalledTimes(1);
      expect(control.checking).toBe(true);
      await act(async () => {
        emitMenu();
      });
      expect(deps.check).toHaveBeenCalledTimes(1);
      await act(async () => pending.resolve(null));
      await flush();
      expect(control.checking).toBe(false);
      expect(deps.check).toHaveBeenCalledTimes(1);
    });

    it("the_menu_entry_checks_like_checkNow", async () => {
      const deps = quiet({ check: vi.fn(async () => NEW) });
      await mount(deps);
      expect(deps.onMenu).toHaveBeenCalledTimes(1);
      await act(async () => emitMenu());
      await flush();
      expect(deps.check).toHaveBeenCalledTimes(1);
      expect(toastOf()?.text).toBe("Gen4 Studio 1.0.2 ist verfügbar.");
    });
  });

  describe("install", () => {
    const found = (over: Partial<UpdateDeps> = {}) => makeDeps({ check: vi.fn(async () => NEW), ...over });

    it("asks_with_version_and_notes_and_installs_on_yes_without_discarding", async () => {
      const deps = found();
      await mount(deps);
      await press("Installieren…");
      expect(deps.ask).toHaveBeenCalledTimes(1);
      const [text, opts] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string, object];
      expect(opts).toEqual({ kind: "info", title: "Gen4 Studio aktualisieren", okLabel: "Installieren und neu starten", cancelLabel: "Abbrechen" });
      expect(text).toContain("Gen4 Studio 1.0.2 installieren");
      expect(text).toContain("Neu:");
      expect(text).toContain("- Fix");
      expect(text).not.toContain("verloren");
      expect(deps.download).toHaveBeenCalledExactlyOnceWith("1.0.2");
      expect(deps.install).toHaveBeenCalledExactlyOnceWith("1.0.2", false);
      // The bytes are downloaded before they are installed.
      expect((deps.download as ReturnType<typeof vi.fn>).mock.invocationCallOrder[0]).toBeLessThan((deps.install as ReturnType<typeof vi.fn>).mock.invocationCallOrder[0]!);
      expect(deps.ask).toHaveBeenCalledTimes(1);
      // The offer is closed once the installation starts.
      expect(toastOf()).toBeUndefined();
    });

    it("builds_the_text_from_the_confirmation_the_warning_and_the_notes", async () => {
      unsaved.current = 1;
      const deps = found();
      await mount(deps);
      await press("Installieren…");
      const [text] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string];
      expect(text).toBe(`${t("app:updates.confirm", { version: "1.0.2", current: "1.0.1" })}\n\n${t("app:updates.unsavedWarning")}\n\n${t("app:updates.notesHeading")}\n- Fix`);
    });

    it("a_no_does_not_install_and_the_next_try_asks_again", async () => {
      const ask = vi.fn(async () => false);
      const deps = found({ ask });
      await mount(deps);
      await press("Installieren…");
      expect(ask).toHaveBeenCalledTimes(1);
      expect(deps.download).not.toHaveBeenCalled();
      expect(deps.install).not.toHaveBeenCalled();
      await act(async () => control.checkNow());
      await flush();
      await press("Installieren…");
      expect(ask).toHaveBeenCalledTimes(2);
    });

    it("warns_of_unsaved_changes_and_installs_with_discardUnsaved", async () => {
      unsaved.current = 2;
      const deps = found();
      await mount(deps);
      await press("Installieren…");
      const [text, opts] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string, { kind: string }];
      expect(opts.kind).toBe("warning");
      expect(text).toContain("Ungespeicherte Änderungen gehen verloren.");
      expect(deps.ask).toHaveBeenCalledTimes(1);
      expect(deps.download).toHaveBeenCalledExactlyOnceWith("1.0.2");
      expect(deps.install).toHaveBeenCalledExactlyOnceWith("1.0.2", true);
    });

    it("a_no_with_unsaved_changes_installs_nothing", async () => {
      unsaved.current = 2;
      const deps = found({ ask: vi.fn(async () => false) });
      await mount(deps);
      await press("Installieren…");
      expect(deps.download).not.toHaveBeenCalled();
      expect(deps.install).not.toHaveBeenCalled();
    });

    it("asks_again_when_unsaved_changes_appear_during_the_download", async () => {
      const ask = vi.fn(async () => true);
      const deps = found({
        ask,
        download: vi.fn(async () => {
          unsaved.current = 1;
        }),
      });
      await mount(deps);
      await press("Installieren…");
      expect(ask).toHaveBeenCalledTimes(2);
      const [first, firstOpts] = ask.mock.calls[0] as unknown as [string, { kind: string }];
      const [second, secondOpts] = ask.mock.calls[1] as unknown as [string, object];
      expect(firstOpts.kind).toBe("info");
      expect(first).not.toContain("verloren");
      expect(secondOpts).toEqual({ kind: "warning", title: "Gen4 Studio aktualisieren", okLabel: "Installieren und neu starten", cancelLabel: "Abbrechen" });
      expect(second).toBe(`${t("app:updates.confirm", { version: "1.0.2", current: "1.0.1" })}\n\n${t("app:updates.unsavedWarning")}\n\n${t("app:updates.notesHeading")}\n- Fix`);
      expect(deps.install).toHaveBeenCalledExactlyOnceWith("1.0.2", true);
    });

    it("a_no_to_the_second_question_removes_the_progress_and_installs_nothing", async () => {
      const ask = vi.fn().mockResolvedValueOnce(true).mockResolvedValueOnce(false);
      const deps = found({
        ask,
        download: vi.fn(async () => {
          unsaved.current = 1;
        }),
      });
      await mount(deps);
      await press("Installieren…");
      expect(ask).toHaveBeenCalledTimes(2);
      expect(deps.install).not.toHaveBeenCalled();
      expect(listToasts().filter((x) => x.kind === "progress")).toEqual([]);
      expect(listToasts().filter((x) => x.kind === "error")).toEqual([]);
      // The lock is gone: a new try asks again.
      unsaved.current = 0;
      await act(async () => control.checkNow());
      await flush();
      await press("Installieren…");
      expect(ask).toHaveBeenCalledTimes(3);
    });

    it("does_not_ask_again_when_the_unsaved_changes_did_not_grow", async () => {
      unsaved.current = 2;
      const deps = found({
        download: vi.fn(async () => {
          unsaved.current = 1;
        }),
      });
      await mount(deps);
      await press("Installieren…");
      expect(deps.ask).toHaveBeenCalledTimes(1);
      expect(deps.install).toHaveBeenCalledExactlyOnceWith("1.0.2", true);
    });

    it("cuts_long_notes_to_1500_characters_and_an_ellipsis", async () => {
      const deps = found({ check: vi.fn(async () => ({ ...NEW, notes: "x".repeat(2000) })) });
      await mount(deps);
      await press("Installieren…");
      const [text] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string];
      expect(text).toContain(`${"x".repeat(1500)}…`);
      expect(text).not.toContain("x".repeat(1501));
    });

    it("keeps_notes_of_exactly_1500_characters_whole", async () => {
      const deps = found({ check: vi.fn(async () => ({ ...NEW, notes: "y".repeat(1500) })) });
      await mount(deps);
      await press("Installieren…");
      const [text] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string];
      expect(text.endsWith("y".repeat(1500))).toBe(true);
      expect(text).not.toContain("…");
    });

    it("leaves_out_the_notes_heading_without_notes", async () => {
      const deps = found({ check: vi.fn(async () => ({ ...NEW, notes: null })) });
      await mount(deps);
      await press("Installieren…");
      const [text] = (deps.ask as ReturnType<typeof vi.fn>).mock.calls[0] as [string];
      expect(text).toBe(t("app:updates.confirm", { version: "1.0.2", current: "1.0.1" }));
    });

    it("shows_the_progress_in_the_toast_with_percent_bytes_and_installing", async () => {
      await mount(found());
      await act(async () => emitProgress({ phase: "downloading", done: 50, total: 100 }));
      expect(toastOf("progress")?.text).toBe("Update wird geladen… 50 %");
      await act(async () => emitProgress({ phase: "downloading", done: 1999, total: 4000 }));
      expect(toastOf("progress")?.text).toBe("Update wird geladen… 49 %");
      await act(async () => emitProgress({ phase: "downloading", done: 123_456, total: null }));
      expect(toastOf("progress")?.text).toBe(`Update wird geladen… ${formatBytes(123_456)}`);
      await act(async () => emitProgress({ phase: "downloading", done: 2048, total: 0 }));
      expect(toastOf("progress")?.text).toBe(`Update wird geladen… ${formatBytes(2048)}`);
      await act(async () => emitProgress({ phase: "installing" }));
      expect(toastOf("progress")?.text).toBe("Update wird installiert…");
      expect(listToasts().filter((x) => x.kind === "progress")).toHaveLength(1);
    });

    it("closes_the_progress_and_shows_an_error_toast_when_the_download_fails", async () => {
      const deps = found({ download: vi.fn(async () => Promise.reject({ code: "error", message: "Verbindung abgebrochen", asset: null })) });
      await mount(deps);
      await press("Installieren…");
      expect(deps.install).not.toHaveBeenCalled();
      expect(listToasts().filter((x) => x.kind === "progress")).toEqual([]);
      expect(listToasts().map((x) => [x.kind, x.text])).toEqual([["error", "Das Update ließ sich nicht installieren: Verbindung abgebrochen"]]);
    });

    it("closes_the_progress_and_shows_an_error_toast_when_the_install_fails", async () => {
      const deps = found({ install: vi.fn(async () => Promise.reject({ code: "unsaved", message: "Es gibt ungespeicherte Änderungen.", asset: null })) });
      await mount(deps);
      await press("Installieren…");
      expect(listToasts().filter((x) => x.kind === "progress")).toEqual([]);
      expect(listToasts().map((x) => [x.kind, x.text])).toEqual([["error", "Das Update ließ sich nicht installieren: Es gibt ungespeicherte Änderungen."]]);
    });

    it("a_failed_install_can_be_tried_again_with_a_new_check", async () => {
      const install = vi.fn().mockRejectedValueOnce(new Error("kaputt")).mockResolvedValue(undefined);
      const deps = found({ install });
      await mount(deps);
      await press("Installieren…");
      await act(async () => control.checkNow());
      await flush();
      await press("Installieren…");
      expect(install).toHaveBeenCalledTimes(2);
    });

    it("two_quick_installs_run_one_installation", async () => {
      const pending = deferred<boolean>();
      const ask = vi.fn(() => pending.promise);
      const deps = found({ ask });
      await mount(deps);
      const action = toastOf()!.actions.find((a) => a.label === "Installieren…")!;
      await act(async () => {
        action.onClick();
        action.onClick();
      });
      await act(async () => pending.resolve(true));
      await flush();
      expect(ask).toHaveBeenCalledTimes(1);
      expect(deps.download).toHaveBeenCalledTimes(1);
      expect(deps.install).toHaveBeenCalledTimes(1);
    });

    it("a_second_install_while_the_download_or_the_install_runs_does_nothing", async () => {
      const downloading = deferred<void>();
      const running = deferred<void>();
      const deps = found({ download: vi.fn(() => downloading.promise), install: vi.fn(() => running.promise) });
      await mount(deps);
      const action = toastOf()!.actions.find((a) => a.label === "Installieren…")!;
      await act(async () => action.onClick());
      await flush();
      await act(async () => action.onClick());
      await flush();
      expect(deps.download).toHaveBeenCalledTimes(1);
      expect(deps.ask).toHaveBeenCalledTimes(1);
      await act(async () => downloading.resolve());
      await flush();
      await act(async () => action.onClick());
      await flush();
      expect(deps.install).toHaveBeenCalledTimes(1);
      expect(deps.download).toHaveBeenCalledTimes(1);
      expect(deps.ask).toHaveBeenCalledTimes(1);
      // A check from the menu waits too.
      await act(async () => emitMenu());
      expect(deps.check).toHaveBeenCalledTimes(1);
      await act(async () => running.resolve());
      await flush();
      // Nothing else happens in the frontend: the backend restarts the app.
      expect(listToasts().filter((x) => x.kind === "error")).toEqual([]);
    });

    it("a_check_from_the_menu_during_the_question_does_not_check", async () => {
      const pending = deferred<boolean>();
      const deps = found({ ask: vi.fn(() => pending.promise) });
      await mount(deps);
      await act(async () => toastOf()!.actions[0]!.onClick());
      await act(async () => emitMenu());
      expect(deps.check).toHaveBeenCalledTimes(1);
      await act(async () => pending.resolve(false));
      await flush();
      await act(async () => emitMenu());
      await flush();
      expect(deps.check).toHaveBeenCalledTimes(2);
    });
  });

  it("removes_its_listeners_when_it_unmounts", async () => {
    await mount(makeDeps());
    expect(unlistenProgress).not.toHaveBeenCalled();
    act(() => root.unmount());
    expect(unlistenProgress).toHaveBeenCalledTimes(1);
    expect(unlistenMenu).toHaveBeenCalledTimes(1);
    root = createRoot(container);
  });
});
