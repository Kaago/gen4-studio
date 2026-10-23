import { useCallback, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { acceptDisk, buildRom, dirty, errorText, headerPatchStatus, isCmdError, listHeaders, memberCounts, openProject, reloadAsset, repairHeaderPatch, runRom, save } from "../api";
import type { AssetId, HeaderListing, HeaderSummary, MemberCounts, ProjectInfo } from "../api";
import { staleText } from "../assets";
import { ask, openPath, savePath } from "../platform/dialogs";
import { inTauri } from "../platform/transport";
import type { Message } from "./message";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";
import type { ScriptBridge } from "../workbench/script/useScripts";
import { eventReadOnlyReason, lockedText } from "../world/events";

const EMULATOR_KEY = "g4.emulatorPath";

function storedEmulator(): string | null {
  try {
    return localStorage.getItem(EMULATOR_KEY);
  } catch {
    return null;
  }
}

function storeEmulator(path: string) {
  try {
    localStorage.setItem(EMULATOR_KEY, path);
  } catch {
    /* the choice is simply asked again next time */
  }
}

interface SessionParams {
  /** Where a saved text archive is announced (the language server reads it again). */
  bus: EditBus;
  /** Where the problems of a save or a build go. */
  problems: ProblemsStore;
  /** The open scripts, which a build saves first and the unsaved-edit guards count. A ref: the scripts hook is created after this one (it needs `info`). */
  scripts: MutableRefObject<ScriptBridge>;
  /** Brings the problems list to the front; the message of a build that failed to compile offers it as a button. */
  onShowProblems?: () => void;
}

/**
 * The project as a whole: opening, saving, building and running it, the unsaved-edit count, the
 * message bar and the handling of a file that changed on disk (`stale`). What a stale resolution
 * has to reload belongs to the scene and the editors; `staleRefresh` is where the app plugs that
 * in (a ref, because the hooks that know how are created after this one).
 */
export function useProjectSession({ bus, problems, scripts, onShowProblems }: SessionParams) {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);
  /** When the last ROM was built (null: none since this project was opened). */
  const [lastBuiltAt, setLastBuiltAt] = useState<number | null>(null);
  const [counts, setCounts] = useState<MemberCounts>({});

  // The close guard is registered once, so it reads the dirty count through a ref.
  const dirtyRef = useRef(0);
  dirtyRef.current = dirtyCount;

  const staleRefresh = useRef<() => Promise<void>>(async () => {});

  const refreshDirty = useCallback(async () => {
    try {
      setDirtyCount((await dirty()).length);
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, []);

  // Stable so the memoised scene is not re-rendered by every hover-driven inspector update.
  const onEdited = useCallback(() => void refreshDirty(), [refreshDirty]);

  // The scene's progress line ("Modelle 37/900") lives in the message bar, but must not wipe
  // an unrelated message: progress only replaces nothing or its own earlier line, and clearing
  // only removes its own line. A failure summary (kind "error") always shows.
  const loadMessage = useRef<Message | null>(null);
  const onLoadStatus = useCallback((next: Message | null) => {
    const previous = loadMessage.current;
    loadMessage.current = next;
    setMessage((shown) => {
      const own = shown === null || shown === previous;
      if (next === null) return own ? null : shown;
      return own || next.kind === "error" ? next : shown;
    });
  }, []);

  const fail = useCallback((e: unknown) => {
    // A locked event file says why it cannot be changed, in the words the inspector uses.
    const locked = eventReadOnlyReason(e);
    if (locked !== null) {
      setMessage({ kind: "error", text: lockedText(locked) });
      return;
    }
    if (isCmdError(e) && e.code === "stale" && e.asset) {
      const asset: AssetId = e.asset;
      const resolve = (action: (a: AssetId) => Promise<void>, done: string) => async () => {
        try {
          await action(asset);
          await staleRefresh.current();
          setMessage({ kind: "info", text: done });
        } catch (err) {
          setMessage({ kind: "error", text: errorText(err) });
        }
      };
      setMessage({
        kind: "error",
        text: staleText(asset),
        actions: [
          { label: "Neu laden (eigene Änderung verwerfen)", run: () => void resolve(reloadAsset, "Von Platte neu geladen.")() },
          { label: "Meine Version behalten", run: () => void resolve(acceptDisk, "Eigene Version behalten, bitte erneut speichern.")() },
        ],
      });
      return;
    }
    setMessage({ kind: "error", text: errorText(e) });
  }, []);

  // New members change the upper bounds of the header editor's map and matrix fields.
  const onCountsChanged = useCallback(async () => {
    try {
      setCounts(await memberCounts());
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, []);

  /**
   * Asks to discard unsaved edits, then opens the project at `known` or, without one, the folder the user picks;
   * `resetProject` swaps the scene's and the editors' state for the new project's header list.
   * "cancelled": the user said no (or picked nothing); "failed": the project could not be opened (the message says why).
   */
  async function onOpen(resetProject: (headers: HeaderSummary[]) => void, known?: string): Promise<"opened" | "cancelled" | "failed"> {
    try {
      if (dirtyCount + scripts.current.dirtyFiles().length > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) return "cancelled";
      const path = known ?? (await openPath({ directory: true }));
      if (path === null) return "cancelled";
      return (await openProjectAt(path, resetProject)) ? "opened" : "failed";
    } catch (e) {
      fail(e);
      return "failed";
    }
  }

  /** Opens the project at `path` (no question asked); see `onOpen` for `resetProject`. False when it could not be opened. */
  async function openProjectAt(path: string, resetProject: (headers: HeaderSummary[]) => void): Promise<boolean> {
    try {
      const opened = await openProject(path);
      // The backend now has the new project. Fetch its listing first, then swap the whole UI
      // state at once; if the listing fails the UI shows the new, empty project instead of
      // keeping the old project's headers and matrix.
      let listing: HeaderListing | null = null;
      let listError: unknown = null;
      try {
        listing = await listHeaders();
      } catch (e) {
        listError = e;
      }
      let loadedCounts: MemberCounts = {};
      let countsError: unknown = null;
      try {
        loadedCounts = await memberCounts();
      } catch (e) {
        countsError = e;
      }
      resetProject(listing?.headers ?? []);
      setCounts(loadedCounts);
      setInfo(opened);
      setLastRom(null);
      setLastBuiltAt(null);
      if (listError !== null) fail(listError);
      else if (countsError !== null) fail(countsError);
      else setMessage(listing?.names_error ? { kind: "error", text: `Ortsnamen nicht lesbar: ${listing.names_error}` } : null);
      await offerPatchRepair();
      await refreshDirty();
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }

  /**
   * A project whose Dynamic Headers patch calls a wrong function builds a ROM that hangs after the intro: say so when
   * it opens and offer the repair. A state that cannot be read says nothing here (the wizard's header step shows it).
   */
  async function offerPatchRepair() {
    let status;
    try {
      status = await headerPatchStatus();
    } catch {
      return;
    }
    if (!status?.repairable) return;
    const repair = async () => {
      try {
        await repairHeaderPatch();
        setMessage({ kind: "info", text: "Patch „Dynamische Header“ repariert: arm9.bin geändert, bitte ROM neu bauen." });
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      }
    };
    setMessage({
      kind: "error",
      text: "Patch „Dynamische Header“ fehlerhaft: das Spiel bleibt nach dem Intro schwarz (falscher Funktionsaufruf, wie ihn DSPRE für Platin DE/FR/IT/ES schreibt).",
      actions: [{ label: "Patch reparieren", run: () => void repair() }],
    });
  }

  /**
   * Saves everything dirty in the store. Text warnings go to the problems store; the language server is told about the archives written. A header
   * table save compiles what it made stale (a script whose text literal resolves only now): those files' problems are replaced, and the text panel
   * reloads the archives that compile appended to.
   */
  async function saveStore() {
    const report = await save();
    const archives = [...new Set([...report.written.filter((a) => a.kind === "TextArchive").map((a) => a.index), ...report.touched_archives])];
    // An archive or a source that was written without any problem is cleared.
    problems.applyReport(report.problems, { archives, files: report.compiled });
    for (const archive of report.touched_archives) bus.emit({ topic: "textData", archive });
    if (archives.length > 0) bus.emit({ topic: "lspFilesChanged", changes: archives.map((a) => ({ path: `expanded/textArchives/${String(a).padStart(4, "0")}.json`, type: 2 as const })) });
    return report;
  }

  async function onSave() {
    if (!info || busy) return;
    try {
      const report = await saveStore();
      // Open scripts are part of the project: saved the way Cmd+S in the editor saves them (after the archives, which a save refuses to run ahead of).
      const dirtyScripts = scripts.current.dirtyFiles();
      for (const file of dirtyScripts) await scripts.current.save(file);
      setMessage({ kind: "info", text: `${report.written.length + dirtyScripts.length} Datei(en) gespeichert` });
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onBuild() {
    try {
      // The system dialog keeps the bare name; only the in-page dialog of the browser mode is prefilled.
      const out = await savePath({ defaultPath: inTauri() || !info ? "out.nds" : `${info.root}/out.nds` });
      if (!out) return;
      setBusy(true);
      setMessage({ kind: "info", text: "Baue ROM ..." });
      // Open scripts live in the editor, not in the store: save them first, and build nothing when one cannot be saved.
      // A script save refuses while a text archive is unsaved, and the build saves the store only after the scripts: with
      // scripts to save, the store goes first (as in `onSave`).
      const dirtyScripts = scripts.current.dirtyFiles();
      const storeSaved = dirtyScripts.length > 0 ? (await saveStore()).written.length : 0;
      for (const file of dirtyScripts) await scripts.current.save(file);
      const report = await buildRom(out);
      setLastRom(report.rom);
      setLastBuiltAt(Date.now());
      problems.applyBuildReport(report.problems);
      setMessage({ kind: "info", text: `ROM gebaut: ${report.rom} (${report.saved + storeSaved} Datei(en) vorher gespeichert)` });
    } catch (e) {
      if (isCmdError(e) && e.code === "compile") {
        const list = e.problems ?? [];
        problems.applyReport(list);
        const errors = list.filter((p) => p.severity === "error").length;
        // Spec 5.7: the one toast a compile problem gets. The problems themselves go to the store and its counts.
        setMessage(
          errors > 0
            ? { kind: "error", text: `${errors} Fehler in Scripts, Build abgebrochen`, actions: onShowProblems ? [{ label: "Probleme zeigen", run: onShowProblems }] : undefined }
            : { kind: "error", text: `Build abgebrochen: ${e.message}` },
        );
      } else {
        fail(e);
      }
    } finally {
      setBusy(false);
    }
    await refreshDirty();
  }

  async function pickEmulator(): Promise<string | null> {
    const chosen = await openPath({ directory: false, title: "Emulator wählen (z. B. melonDS)" });
    if (chosen === null) return null;
    storeEmulator(chosen);
    return chosen;
  }

  async function onPickEmulator() {
    try {
      const chosen = await pickEmulator();
      if (chosen) setMessage({ kind: "info", text: `Emulator: ${chosen}` });
    } catch (e) {
      fail(e);
    }
  }

  async function onRun() {
    if (!lastRom) return;
    try {
      const emulator = storedEmulator() ?? (await pickEmulator());
      if (!emulator) return;
      await runRom(emulator, lastRom);
      setMessage({ kind: "info", text: `Emulator gestartet: ${lastRom}` });
    } catch (e) {
      fail(e);
    }
  }

  return {
    info,
    message,
    setMessage,
    dirtyCount,
    dirtyRef,
    busy,
    lastRom,
    lastBuiltAt,
    counts,
    staleRefresh,
    refreshDirty,
    onEdited,
    onLoadStatus,
    fail,
    onCountsChanged,
    onOpen,
    openProjectAt,
    onSave,
    onBuild,
    onPickEmulator,
    onRun,
  };
}

export type ProjectSession = ReturnType<typeof useProjectSession>;
