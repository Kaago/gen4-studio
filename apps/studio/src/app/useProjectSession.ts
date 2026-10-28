import { useCallback, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { acceptDisk, buildRom, closeProject as closeBackendProject, createPatch, dirty, errorText, headerPatchStatus, isCmdError, listHeaders, memberCounts, openProject, reloadAsset, repairHeaderPatch, resolveTextConflicts, runRom, save, textConflictArchives, textProjectStatus } from "../api";
import type { AssetId, BuildReport, BuildStep, CmdError, HeaderListing, HeaderSummary, MemberCounts, PatchDto, ProjectInfo, SyncOutcome, TextKeep, TextProjectStatus } from "../api";
import { staleText } from "../assets";
import { formatSeconds } from "../technik/stepWords";
import { BUILD_CONFLICTS, CONFLICTS_TOAST } from "../technik/syncWords";
import { ask, openPath, savePath } from "../platform/dialogs";
import { inTauri } from "../platform/transport";
import type { Message } from "./message";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";
import type { ScriptBridge } from "../workbench/script/useScripts";
import { eventReadOnlyReason, lockedText } from "../world/events";
import { listToasts, toast } from "../ui/frame/toasts";

/** Says that the Dynamic Headers patch calls a wrong function; the toast with this text offers the repair. */
const BROKEN_PATCH = "Patch „Dynamische Header“ fehlerhaft: das Spiel bleibt nach dem Intro schwarz (falscher Funktionsaufruf, wie ihn DSPRE für Platin DE/FR/IT/ES schreibt).";

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

/** „Textarchiv 0012“, „Textarchive 0012, 0045 und 0433“: the archives a conflict names. */
function archivesText(archives: readonly number[]): string {
  const names = archives.map((a) => String(a).padStart(4, "0"));
  if (names.length === 1) return `Textarchiv ${names[0]}`;
  return `Textarchive ${names.slice(0, -1).join(", ")} und ${names[names.length - 1]}`;
}

/**
 * A save met text archives changed as JSON and as member: the save asks nothing itself, the next build does (a ruling of 4.2). Without
 * archives (an error without problems) the backend's message stands.
 */
function saveConflictText(e: CmdError): string {
  const archives = textConflictArchives(e);
  if (archives.length === 0) return e.message;
  const verb = archives.length === 1 ? "wurde" : "wurden";
  return `${archivesText(archives)} ${verb} als Text und als Spieldatei geändert. Wähle beim nächsten „Bauen“, welche Fassung gilt.`;
}

/** What a build of a text project with open conflicts is refused with (the backend's `conflicts` refusal says the same). */
const CONFLICTS: CmdError = { code: "conflicts", message: BUILD_CONFLICTS, asset: null, problems: [] };

/** Ends a sentence of the backend with a full stop. */
const sentence = (text: string) => (/[.!?]$/.test(text) ? text : `${text}.`);

/**
 * A build waiting for the user to choose which version of each archive in conflict applies; it then goes to `out` with the same `fresh`.
 * `play`: the build came from „Hier spielen“, so the ROM is started after it. `resolving`: the choice is being written (the dialog waits).
 */
export type TextConflicts = { archives: number[]; out: string; fresh: boolean; play: boolean; resolving: boolean };

function forgetEmulator() {
  try {
    localStorage.removeItem(EMULATOR_KEY);
  } catch {
    /* nothing stored that could be kept */
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
  /**
   * The sync of a text project with DSPRE, run before a build (null outside a text project). A ref: the sync hook is created after this
   * one (it needs `info` and `busy`).
   */
  syncBeforeBuild?: MutableRefObject<(() => Promise<SyncOutcome | null>) | null>;
  /** Shows Technik › Textprojekt; the message of a build stopped by conflicts offers it („Lösen…“). */
  onShowConflicts?: () => void;
}

/**
 * The project as a whole: opening, saving, building and running it, the unsaved-edit count, the
 * message bar and the handling of a file that changed on disk (`stale`). What a stale resolution
 * has to reload belongs to the scene and the editors; `staleRefresh` is where the app plugs that
 * in (a ref, because the hooks that know how are created after this one).
 */
export function useProjectSession({ bus, problems, scripts, onShowProblems, syncBeforeBuild, onShowConflicts }: SessionParams) {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  // The assets with unsaved changes (the inspector badges the record it shows); the same array while the list did not change.
  const [dirtyAssets, setDirtyAssets] = useState<readonly AssetId[]>([]);
  const [busy, setBusy] = useState(false);
  // `busy` as `exclusive` reads it before the next render.
  const busyRef = useRef(false);
  // `busy` comes from `exclusive` (a set-up, take-back or lift), not from a build: the status line does not say „Baut …“ then.
  const [holding, setHolding] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);
  /** When the last ROM was built (null: none since this project was opened). */
  const [lastBuiltAt, setLastBuiltAt] = useState<number | null>(null);
  /** The steps of the last build (null: none since this project was opened). */
  const [lastSteps, setLastSteps] = useState<BuildStep[] | null>(null);
  /** Whether the last build was started with „Neu bauen (ohne Cache)“. */
  const [lastBuildFresh, setLastBuildFresh] = useState(false);
  /** The ROM of the last build when that was the one of „Patch erstellen“ (its steps are the last steps); null when the last build was a normal one. */
  const [lastPatchRom, setLastPatchRom] = useState<string | null>(null);
  /** The error texts the builds of this session showed: a successful build closes their toasts (errors stay until closed otherwise). */
  const buildErrors = useRef(new Set<string>());
  /** A build stopped by text archives in conflict: the dialog asks, `resolveConflicts` builds again. */
  const [textConflicts, setTextConflicts] = useState<TextConflicts | null>(null);
  /** The emulator a ROM starts in (stored for every project); null while none is chosen. */
  const [emulatorPath, setEmulatorPath] = useState<string | null>(() => storedEmulator());
  const [counts, setCounts] = useState<MemberCounts>({});
  /** Bumped whenever the dirty count was read again, i.e. after every write the app makes: the hinge reads the undo history then. */
  const [dirtyRevision, setDirtyRevision] = useState(0);
  /** Bumped after every save of the store and every build (a build saves first): the saved state may differ now (Technik › Änderungen). */
  const [savedRevision, setSavedRevision] = useState(0);

  // The close guard is registered once, so it reads the dirty count through a ref.
  const dirtyRef = useRef(0);
  dirtyRef.current = dirtyCount;

  const staleRefresh = useRef<() => Promise<void>>(async () => {});

  // The info of the session `reloadSession` installed last: the app tells it from an opening by it (`isReopened`).
  const reopenedInfo = useRef<ProjectInfo | null>(null);
  const isReopened = useCallback((shown: ProjectInfo) => reopenedInfo.current === shown, []);
  // The info shown now, for work that runs later than the render that started it.
  const infoRef = useRef<ProjectInfo | null>(info);
  infoRef.current = info;
  // The folder whose session the backend has: set as soon as an opening answered, before the UI has swapped to it (`currentRoot`).
  const backendRoot = useRef<string | null>(null);
  /** The folder the backend's session is of now (null before the first opening); a result for another one is stale. */
  const currentRoot = useCallback(() => backendRoot.current, []);

  /**
   * Reads the unsaved edits again. Every write the app makes ends here, so it announces `edited`: the checks run again (all of them, 300 ms
   * after the last edit). `edited: false` for a read that follows no write of its own whose checks run anyway: a project just opened (the
   * checks run for it as it opens) and a fix (it asks for its run itself), so neither runs the checks twice.
   */
  const refreshDirty = useCallback(async ({ edited = true }: { edited?: boolean } = {}) => {
    try {
      const assets = await dirty();
      setDirtyCount(assets.length);
      setDirtyAssets((before) => (before.length === assets.length && before.every((a, i) => a.kind === assets[i]!.kind && a.index === assets[i]!.index) ? before : assets));
      setDirtyRevision((r) => r + 1);
      if (edited) bus.emit({ topic: "edited" });
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, [bus]);

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
    if (isCmdError(e) && e.code === "text_conflict") {
      setMessage({ kind: "error", text: saveConflictText(e) });
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
      backendRoot.current = opened.root;
      await installSession(opened, resetProject);
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }

  /**
   * The backend opened the session of the open project anew behind the app's back (an agent adopted, took back or lifted the text
   * project): its store, undo history and language server are new. Reads everything again as after opening, without opening once more
   * (that would drop what an agent did since): a new info object, with the text project's state read now, makes the scripts, the
   * language server and the checks start over.
   */
  async function reloadSession(root: string, resetProject: (headers: HeaderSummary[]) => void): Promise<void> {
    // Read when it runs (it waits in the agents' queue): another project opened meanwhile is not reloaded.
    const shown = infoRef.current;
    if (shown === null || shown.root !== root) return;
    let status: TextProjectStatus | undefined;
    let statusError: unknown = null;
    try {
      status = await textProjectStatus();
    } catch (e) {
      statusError = e;
    }
    try {
      const next = { ...shown, text_project: status ?? shown.text_project };
      reopenedInfo.current = next;
      await installSession(next, resetProject);
      // After the install, whose message would replace this one.
      if (statusError !== null) fail(statusError);
    } catch (e) {
      fail(e);
    }
  }

  /**
   * Closes the project (a text project of an older schema the user did not lift): the backend drops its session (so no agent acts on it
   * unseen), then no project shows, as before the first opening. When the backend cannot close it, the project stays shown with the error.
   */
  async function closeProject(resetProject: (headers: HeaderSummary[]) => void): Promise<boolean> {
    try {
      await closeBackendProject();
    } catch (e) {
      fail(e);
      return false;
    }
    backendRoot.current = null;
    reopenedInfo.current = null;
    resetProject([]);
    setCounts({});
    infoRef.current = null;
    setInfo(null);
    setDirtyCount(0);
    setDirtyAssets([]);
    setLastRom(null);
    setLastBuiltAt(null);
    setLastSteps(null);
    setLastBuildFresh(false);
    setLastPatchRom(null);
    setTextConflicts(null);
    setMessage(null);
    return true;
  }

  /**
   * Runs `work` (setting the project up as text project, taking it back, lifting it) with the session `busy`, like a build: open, save,
   * build, undo and redo wait, and so do the set-up's and take-back's buttons. Throws on without running when something else holds it.
   */
  async function exclusive<T>(work: () => Promise<T>): Promise<T> {
    if (busyRef.current || busy) throw new Error("Das Projekt ist gerade beschäftigt.");
    busyRef.current = true;
    setBusy(true);
    setHolding(true);
    try {
      return await work();
    } finally {
      busyRef.current = false;
      setBusy(false);
      setHolding(false);
    }
  }

  /** Swaps the UI's state for the session the backend now has (`opened`); see `onOpen` for `resetProject`. */
  async function installSession(opened: ProjectInfo, resetProject: (headers: HeaderSummary[]) => void): Promise<void> {
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
    infoRef.current = opened;
    setInfo(opened);
    setLastRom(null);
    setLastBuiltAt(null);
    setLastSteps(null);
    setLastBuildFresh(false);
    setLastPatchRom(null);
    setTextConflicts(null);
    if (listError !== null) fail(listError);
    else if (countsError !== null) fail(countsError);
    else setMessage(listing?.names_error ? { kind: "error", text: `Ortsnamen nicht lesbar: ${listing.names_error}` } : null);
    await offerPatchRepair();
    // The checks run for the opened project by themselves (`useChecks`): this read is no edit.
    await refreshDirty({ edited: false });
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
        patchRepaired();
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      }
    };
    setMessage({ kind: "error", text: BROKEN_PATCH, actions: [{ label: "Patch reparieren", run: () => void repair() }] });
  }

  /**
   * The patch was repaired (from the toast, in Technik › Patches or in a wizard's header step): the error toast that offered the repair goes
   * (an error toast outlives its message, see `useToastMirror`), and the session says what changed and what to do next.
   */
  const patchRepaired = useCallback(() => {
    for (const shown of listToasts()) if (shown.kind === "error" && shown.text === BROKEN_PATCH) toast.dismiss(shown.id);
    setMessage({ kind: "info", text: "Patch „Dynamische Header“ repariert: arm9.bin geändert, bitte ROM neu bauen." });
  }, []);

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
    setSavedRevision((r) => r + 1);
    await refreshDirty();
  }

  /**
   * The one build path of the session: `busy` while it runs (the app's open, save and build guards read it), the open scripts saved first
   * (they live in the editor, not in the store; a script save refuses while a text archive is unsaved, so with scripts to save the store goes
   * first, as in `onSave`), then `run`, which saves the store and builds (`build_rom`, or `create_patch`, which builds before it writes the
   * patch). After it the build's time, steps and problems are the session's (`fresh`: it ran without the cache), its ROM becomes the last ROM
   * only with `ownsRom` (a patch's ROM is not „Hier spielen“'s target: building there would change the ROM its notes name; it is remembered as
   * `lastPatchRom` so the steps say whose they are), and the unsaved edits are read again. A compile failure puts its problems into the store;
   * every failure is thrown on. Answers what `run` answered and how many store files were saved ahead of the scripts.
   */
  async function guardedBuild<R>(
    run: () => Promise<R>,
    reportOf: (result: R) => BuildReport,
    ownsRom: boolean,
    fresh = false,
  ): Promise<{ result: R; storeSaved: number }> {
    setBusy(true);
    try {
      // A text project takes over what changed outside first (its progress and messages are the sync's); with an open conflict nothing is
      // built. The backend syncs once more before it builds: that one finds this sync's record and only stats the files.
      const sync = syncBeforeBuild?.current;
      if (sync) {
        const outcome = await sync();
        if (outcome !== null && outcome.conflicts.length > 0) throw CONFLICTS;
      }
      const dirtyScripts = scripts.current.dirtyFiles();
      const storeSaved = dirtyScripts.length > 0 ? (await saveStore()).written.length : 0;
      for (const file of dirtyScripts) await scripts.current.save(file);
      const result = await run();
      const report = reportOf(result);
      if (ownsRom) setLastRom(report.rom);
      setLastBuiltAt(Date.now());
      setLastSteps(report.steps);
      setLastBuildFresh(fresh);
      setLastPatchRom(ownsRom ? null : report.rom);
      problems.applyBuildReport(report.problems);
      return { result, storeSaved };
    } catch (e) {
      if (isCmdError(e) && e.code === "compile") problems.applyReport(e.problems ?? []);
      throw e;
    } finally {
      setBusy(false);
      setSavedRevision((r) => r + 1);
      await refreshDirty();
    }
  }

  /**
   * Builds the ROM (saving first). Without `known` the output path is asked for; with it (the path of an earlier build) no dialog opens.
   * `fresh` („Neu bauen (ohne Cache)“) drops the build state first, so every step runs fully. `play`: the build of „Hier spielen“ (kept
   * with a text conflict, so the caller of `resolveConflicts` can start the ROM). Resolves to the ROM's path, or null when nothing was built
   * (cancelled, failed to compile or to build, or stopped by text archives in conflict: then `textConflicts` asks).
   */
  async function onBuild(known?: string, fresh = false, play = false): Promise<string | null> {
    // A text project with an open conflict builds nothing: that is said before the path is asked for (the build's own sync checks again).
    if (known === undefined && (await conflictsOpen())) {
      buildError(conflictsMessage());
      return null;
    }
    let out: string | null;
    try {
      // The system dialog keeps the bare name; only the in-page dialog of the browser mode is prefilled.
      out = known ?? (await savePath({ defaultPath: inTauri() || !info ? "out.nds" : `${info.root}/out.nds` }));
    } catch (e) {
      fail(e);
      return null;
    }
    if (!out) return null;
    return buildTo(out, fresh, play, false);
  }

  /** Whether the sync of a text project finds a conflict open (false outside a text project, or when the sync failed). */
  async function conflictsOpen(): Promise<boolean> {
    const sync = syncBeforeBuild?.current;
    if (!sync) return false;
    const outcome = await sync();
    return outcome !== null && outcome.conflicts.length > 0;
  }

  /** A build stopped by conflicts: one message with „Lösen…“, in the place of the sync's conflict message (not stacked below it). */
  function conflictsMessage(): Message {
    return { kind: "error", text: BUILD_CONFLICTS, key: CONFLICTS_TOAST, actions: onShowConflicts ? [{ label: "Lösen…", run: onShowConflicts }] : undefined };
  }

  /** Shows an error of a build; the next successful build closes its toast. */
  function buildError(shown: Message) {
    buildErrors.current.add(shown.text);
    setMessage(shown);
  }

  /** Closes the error toasts of earlier builds that are still open: the build they were about went through now. */
  function closeBuildErrors() {
    for (const shown of listToasts()) if (shown.kind === "error" && buildErrors.current.has(shown.text)) toast.dismiss(shown.id);
    buildErrors.current.clear();
  }

  /**
   * Saves and builds to `out` on the session's build path. `resolved`: the build after a text conflict was resolved; a conflict now asks no
   * second time.
   */
  async function buildTo(out: string, fresh: boolean, play: boolean, resolved: boolean): Promise<string | null> {
    try {
      setMessage({ kind: "info", text: "Baue ROM ..." });
      const { result: report, storeSaved } = await guardedBuild(() => buildRom(out, fresh), (r) => r, true, fresh);
      closeBuildErrors();
      setMessage({ kind: "info", text: `ROM gebaut: ${report.rom} in ${formatSeconds(report.ms)} (${report.saved + storeSaved} Datei(en) vorher gespeichert)` });
      return report.rom;
    } catch (e) {
      if (isCmdError(e) && e.code === "compile") {
        const list = e.problems ?? [];
        const errors = list.filter((p) => p.severity === "error").length;
        // Hidden files under files/ (source "build"): the backend's message counts them apart and says how to remove them.
        const hidden = list.some((p) => p.severity === "error" && (p.source === "build" || p.message.startsWith("build.hidden-file")));
        const actions = onShowProblems ? [{ label: "Probleme zeigen", run: onShowProblems }] : undefined;
        // Spec 5.7: the one toast a compile problem gets. The problems themselves went to the store and its counts.
        buildError(
          hidden
            ? { kind: "error", text: sentence(e.message), actions }
            : errors > 0
              ? { kind: "error", text: `${errors} Fehler in Scripts, Build abgebrochen`, actions }
              : { kind: "error", text: `Build abgebrochen: ${e.message}` },
        );
      } else if (isCmdError(e) && e.code === "conflicts") {
        // Conflicts between the Studio and DSPRE (found by the sync before the build, or by the backend's own): the way out is Technik.
        buildError(conflictsMessage());
      } else if (isCmdError(e) && e.code === "text_conflict" && textConflictArchives(e).length > 0) {
        const archives = textConflictArchives(e);
        if (resolved) {
          // Changed again since the choice (or another archive): no loop of questions; the next „Bauen“ asks afresh.
          const verb = archives.length === 1 ? "ist" : "sind";
          buildError({ kind: "error", text: `Build abgebrochen: ${archivesText(archives)} ${verb} nach dem Auflösen wieder auf beiden Seiten geändert. Baue erneut, dann fragt das Studio noch einmal.` });
        } else {
          setMessage(null);
          setTextConflicts({ archives, out, fresh, play, resolving: false });
        }
      } else {
        // The plain error of a build (a stale or locked asset of the save keeps its own words and way out).
        buildErrors.current.add(errorText(e));
        fail(e);
      }
      return null;
    }
  }

  /**
   * „Patch erstellen“ (Technik › Veröffentlichen) on the session's build path: saving, Bauen and opening wait for it, and its build counts
   * as the last build (time, steps, problems), but the ROM it built (`build/<Name>.nds`, named with its SHA-1 in the notes) does not become
   * the last ROM, so „Hier spielen“ never builds over it. A failure is thrown on: the patch card says why.
   */
  async function buildPatch(): Promise<PatchDto> {
    return (await guardedBuild(createPatch, (dto) => dto.build, false)).result;
  }

  /**
   * „Übernehmen und bauen“ of the text conflict: resolves each archive as chosen, then builds again to the same path with the same `fresh`.
   * While the choice is written the dialog stays open and waits (`resolving`, the session is busy); it closes when the build starts. A refused
   * resolve says why and builds nothing. Resolves to the ROM's path, or null when nothing was built; whether to start it is `play` of the
   * conflict.
   */
  async function resolveConflicts(choices: { archive: number; keep: TextKeep }[]): Promise<string | null> {
    const pending = textConflicts;
    if (pending === null || pending.resolving) return null;
    setTextConflicts({ ...pending, resolving: true });
    setBusy(true);
    let archives: number[];
    try {
      archives = await resolveTextConflicts(choices);
    } catch (e) {
      setTextConflicts(null);
      setBusy(false);
      fail(e);
      await refreshDirty();
      return null;
    }
    setTextConflicts(null);
    // The text panel and the language server read the overwritten archives again.
    for (const archive of archives) bus.emit({ topic: "textData", archive });
    if (archives.length > 0) bus.emit({ topic: "lspFilesChanged", changes: archives.map((a) => ({ path: `expanded/textArchives/${String(a).padStart(4, "0")}.json`, type: 2 as const })) });
    return buildTo(pending.out, pending.fresh, pending.play, true);
  }

  /** „Abbrechen“ of the text conflict: nothing is built, nothing is overwritten. */
  function cancelConflicts() {
    setTextConflicts(null);
    setMessage({ kind: "info", text: "Nichts gebaut. Beim nächsten „Bauen“ fragt das Studio wieder, welche Fassung gilt." });
  }

  async function pickEmulator(): Promise<string | null> {
    const chosen = await openPath({ directory: false, title: "Emulator wählen (z. B. melonDS)" });
    if (chosen === null) return null;
    storeEmulator(chosen);
    setEmulatorPath(chosen);
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

  /** Starts the emulator on `rom` (default: the last ROM built); asks for an emulator first when none is stored. */
  async function onRun(rom: string | null = lastRom) {
    if (!rom) return;
    try {
      const emulator = storedEmulator() ?? (await pickEmulator());
      if (!emulator) return;
      try {
        await runRom(emulator, rom);
      } catch (e) {
        // A wrong choice cannot be undone anywhere else in the UI: forget it, so the next start asks again.
        forgetEmulator();
        setEmulatorPath(null);
        const why = errorText(e);
        setMessage({ kind: "error", text: `${/[.!?]$/.test(why) ? why : `${why}.`} Beim nächsten Start fragt das Studio erneut nach dem Emulator.` });
        return;
      }
      setMessage({ kind: "info", text: `Emulator gestartet: ${rom}` });
    } catch (e) {
      fail(e);
    }
  }

  return {
    info,
    message,
    setMessage,
    dirtyCount,
    dirtyAssets,
    dirtyRef,
    busy,
    /** A build (or a patch's build) runs; `busy` without one is a set-up, take-back or lift. */
    building: busy && !holding,
    lastRom,
    lastBuiltAt,
    lastSteps,
    lastBuildFresh,
    lastPatchRom,
    textConflicts,
    emulatorPath,
    counts,
    dirtyRevision,
    savedRevision,
    staleRefresh,
    refreshDirty,
    onEdited,
    onLoadStatus,
    fail,
    onCountsChanged,
    onOpen,
    openProjectAt,
    reloadSession,
    closeProject,
    exclusive,
    isReopened,
    currentRoot,
    onSave,
    onBuild,
    buildPatch,
    resolveConflicts,
    cancelConflicts,
    onPickEmulator,
    onRun,
    patchRepaired,
  };
}

export type ProjectSession = ReturnType<typeof useProjectSession>;
