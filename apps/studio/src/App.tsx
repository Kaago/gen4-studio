import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  acceptDisk,
  buildRom,
  dirty,
  errorText,
  getMatrix,
  isCmdError,
  listHeaders,
  openProject,
  redo,
  reloadAsset,
  runRom,
  save,
  undo,
} from "./api";
import type { AssetId, HeaderListing, HeaderSummary, MatrixDto, ProjectInfo } from "./api";
import { staleText } from "./assets";
import { Toolbar } from "./Toolbar";
import type { Message } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import type { WorkbenchData } from "./workbench/Workbench";
import type { InspectorModel } from "./workbench/Inspector";

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

export default function App() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  const [reloadToken, setReloadToken] = useState(0);
  const [busy, setBusy] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);
  const [inspector, setInspector] = useState<InspectorModel>({ kind: "none" });

  // The close guard is registered once, so it reads the dirty count through a ref.
  const dirtyRef = useRef(0);
  dirtyRef.current = dirtyCount;

  const refreshDirty = useCallback(async () => {
    try {
      setDirtyCount((await dirty()).length);
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, []);

  // Stable so the memoised scene is not re-rendered by every hover-driven inspector update.
  const onEdited = useCallback(() => void refreshDirty(), [refreshDirty]);

  /** Reloads the shown matrix (a new object makes the scene refetch all its maps) and the dirty count. */
  const refreshAfterStale = useCallback(
    async (matrixIndex: number | null) => {
      try {
        if (matrixIndex !== null) setMatrix(await getMatrix(matrixIndex));
      } finally {
        // The dirty count must follow the reload/accept even when refetching the matrix fails.
        await refreshDirty();
      }
    },
    [refreshDirty],
  );

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

  const matrixIndexRef = useRef<number | null>(null);
  matrixIndexRef.current = matrix?.index ?? null;

  const fail = useCallback(
    (e: unknown) => {
      if (isCmdError(e) && e.code === "stale" && e.asset) {
        const asset: AssetId = e.asset;
        const resolve = (action: (a: AssetId) => Promise<void>, done: string) => async () => {
          try {
            await action(asset);
            await refreshAfterStale(matrixIndexRef.current);
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
    },
    [refreshAfterStale],
  );

  async function onOpen() {
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
      selectSeq.current++;
      setHeaders(listing?.headers ?? []);
      setInfo(opened);
      setSelected(null);
      setMatrix(null);
      setLastRom(null);
      if (listError !== null) fail(listError);
      else setMessage(listing?.names_error ? { kind: "error", text: `Ortsnamen nicht lesbar: ${listing.names_error}` } : null);
      await refreshDirty();
    } catch (e) {
      fail(e);
    }
  }

  // The latest header selection (or project switch) wins: a slow getMatrix of an earlier one is
  // dropped, so the matrix never pairs with another header's event file and area data.
  const selectSeq = useRef(0);

  async function onSelect(h: HeaderSummary) {
    const seq = ++selectSeq.current;
    setSelected(h.id);
    try {
      const next = await getMatrix(h.matrix_id);
      if (seq !== selectSeq.current) return;
      setMatrix(next);
      setMessage(null);
    } catch (e) {
      if (seq === selectSeq.current) fail(e);
    }
  }

  async function step(action: typeof undo) {
    if (!info || busy) return;
    try {
      if (await action()) setReloadToken((t) => t + 1);
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
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

  // Closing the window (button, Cmd+Q) asks before discarding unsaved edits.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWindow()
      .onCloseRequested(async (e) => {
        if (dirtyRef.current > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) {
          e.preventDefault();
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Cmd+Q does not close the window first: the backend holds the exit back and asks here.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    let asking = false;
    listen("confirm-quit", async () => {
      if (asking) return;
      asking = true;
      try {
        if (await ask("Ungespeicherte Änderungen verwerfen und beenden?", { kind: "warning" })) {
          await invoke("force_quit");
        }
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      } finally {
        asking = false;
      }
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Shortcuts always call the handlers of the latest render.
  const latest = useRef({ onSave, step });
  latest.current = { onSave, step };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === "z") {
        e.preventDefault();
        void latest.current.step(e.shiftKey ? redo : undo);
      } else if (key === "s") {
        e.preventDefault();
        void latest.current.onSave();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const workbench: WorkbenchData = {
    info,
    headers,
    selected,
    onSelect: (h) => void onSelect(h),
    matrix,
    areaDataId: headers.find((h) => h.id === selected)?.area_data_id ?? 0,
    showPermissions,
    onShowPermissions: setShowPermissions,
    eventFileId: headers.find((h) => h.id === selected)?.event_file_id ?? null,
    showEvents,
    onShowEvents: setShowEvents,
    paintEnabled: !busy,
    reloadToken,
    onEdited,
    onError: fail,
    onLoadStatus,
    inspector,
    setInspector,
    message,
    onDismissMessage: () => setMessage(null),
  };

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", display: "flex", flexDirection: "column", height: "100vh" }}>
      <Toolbar
        projectOpen={info !== null}
        busy={busy}
        dirtyCount={dirtyCount}
        canRun={lastRom !== null}
        onOpen={onOpen}
        onUndo={() => void step(undo)}
        onRedo={() => void step(redo)}
        onSave={() => void onSave()}
        onBuild={() => void onBuild()}
        onRun={() => void onRun()}
        onPickEmulator={() => void onPickEmulator()}
      />
      <Workbench data={workbench} />
    </main>
  );
}
