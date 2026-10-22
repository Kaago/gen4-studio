import { useCallback, useRef, useState } from "react";
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { acceptDisk, buildRom, dirty, errorText, isCmdError, listHeaders, memberCounts, openProject, reloadAsset, runRom, save } from "../api";
import type { AssetId, HeaderListing, HeaderSummary, MemberCounts, ProjectInfo } from "../api";
import { staleText } from "../assets";
import type { Message } from "../Toolbar";
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

/**
 * The project as a whole: opening, saving, building and running it, the unsaved-edit count, the
 * message bar and the handling of a file that changed on disk (`stale`). What a stale resolution
 * has to reload belongs to the scene and the editors; `staleRefresh` is where the app plugs that
 * in (a ref, because the hooks that know how are created after this one).
 */
export function useProjectSession() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);
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

  /** Asks for a project folder and opens it; `resetProject` swaps the scene's and the editors' state for the new project's header list. */
  async function onOpen(resetProject: (headers: HeaderSummary[]) => void) {
    try {
      if (dirtyCount > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) return;
      const path = await open({ directory: true });
      if (typeof path !== "string") return;
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
      if (listError !== null) fail(listError);
      else if (countsError !== null) fail(countsError);
      else setMessage(listing?.names_error ? { kind: "error", text: `Ortsnamen nicht lesbar: ${listing.names_error}` } : null);
      await refreshDirty();
    } catch (e) {
      fail(e);
    }
  }

  async function onSave() {
    if (!info || busy) return;
    try {
      const report = await save();
      setMessage({ kind: "info", text: `${report.written.length} Datei(en) gespeichert` });
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onBuild() {
    try {
      const out = await saveDialog({ defaultPath: "out.nds", filters: [{ name: "ROM", extensions: ["nds"] }] });
      if (!out) return;
      setBusy(true);
      setMessage({ kind: "info", text: "Baue ROM ..." });
      const report = await buildRom(out);
      setLastRom(report.rom);
      setMessage({ kind: "info", text: `ROM gebaut: ${report.rom} (${report.saved} Datei(en) vorher gespeichert)` });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
    await refreshDirty();
  }

  async function pickEmulator(): Promise<string | null> {
    const chosen = await open({ directory: false, multiple: false, title: "Emulator wählen (z. B. melonDS)" });
    if (typeof chosen !== "string") return null;
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
    counts,
    staleRefresh,
    refreshDirty,
    onEdited,
    onLoadStatus,
    fail,
    onCountsChanged,
    onOpen,
    onSave,
    onBuild,
    onPickEmulator,
    onRun,
  };
}

export type ProjectSession = ReturnType<typeof useProjectSession>;
