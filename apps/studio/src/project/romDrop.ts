import { useEffect, useRef, useState } from "react";
import type { DragEvent } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { inTauri } from "../platform/transport";

/** What the wizard "Neues Spiel aus ROM" opens with: a ROM path to read at once, or a note on the step "ROM". */
export type StartWizardRequest = { rom?: string; notice?: string };

/** What a drop carried: one path, one file whose path the page does not know, or nothing usable (also several files). */
export type DropSource = { path: string } | { file: string } | { none: true };

export const REFUSED = "Nur eine .nds-Datei ablegen.";
export const NO_PATH = "Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.";

/** Lines of a list (`text/uri-list` or plain text) without blanks and `#` comments. */
const linesOf = (text: string) =>
  text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l !== "" && !l.startsWith("#"));

/** A path that starts at a root: `/…` or a drive (`C:\…`, `C:/…`). */
const absolute = (p: string) => p.startsWith("/") || /^[A-Za-z]:[\\/]/.test(p);

/** The path of a `file://` URI (`file:///a%20b/x.nds` → `/a b/x.nds`; a drive loses the slash before it); null for another URI. */
function fileUriPath(uri: string): string | null {
  let url: URL;
  try {
    url = new URL(uri);
  } catch {
    return null;
  }
  if (url.protocol !== "file:") return null;
  let path: string;
  try {
    path = decodeURIComponent(url.pathname);
  } catch {
    return null;
  }
  return /^\/[A-Za-z]:\//.test(path) ? path.slice(1) : path;
}

/**
 * The browser's view of a drop: a `file://` URI (some browsers and file managers give one), else an absolute path as plain text, else the
 * name of a single file (a browser never tells its path). Several files, URIs or lines are nothing usable.
 */
export function pathFromDataTransfer(dt: DataTransfer): DropSource {
  const files = dt.files?.length ?? 0;
  if (files > 1) return { none: true };
  const types = [...(dt.types ?? [])];
  if (types.includes("text/uri-list")) {
    const uris = linesOf(dt.getData("text/uri-list"));
    if (uris.length > 1) return { none: true };
    const path = uris.length === 1 ? fileUriPath(uris[0]!) : null;
    if (path !== null) return { path };
  }
  if (types.includes("text/plain")) {
    const lines = linesOf(dt.getData("text/plain"));
    if (lines.length > 1) return { none: true };
    if (lines.length === 1 && absolute(lines[0]!)) return { path: lines[0]! };
  }
  if (files === 1) return { file: dt.files[0]!.name };
  return { none: true };
}

/** Whether a path names a DS ROM (`.nds`, any case). */
export const isRomPath = (p: string) => /\.nds$/i.test(p.trim());

/** Whether a drag may carry a file or a path. */
const carriesPath = (dt: DataTransfer | null) => dt !== null && [...(dt.types ?? [])].some((t) => t === "Files" || t === "text/uri-list" || t === "text/plain");
/** Whether a drag is a file from a file manager (files or a uri list): only that lights the drop frame; a path in plain text may be dropped quietly. */
const carriesFileDrag = (dt: DataTransfer | null) => dt !== null && [...(dt.types ?? [])].some((t) => t === "Files" || t === "text/uri-list");
/** Whether a drag carries files (the browser would open a dropped one in place of the app); a drag of text into a field does not. */
const carriesFiles = (dt: DataTransfer | null) => dt !== null && [...(dt.types ?? [])].includes("Files");

export type RomDrop = {
  /** Something is dragged over the surface that would be taken. */
  over: boolean;
  /** For the element that takes a drop in the browser (in the app the webview reports drops, these do nothing). */
  handlers: {
    onDragEnter: (e: DragEvent) => void;
    onDragOver: (e: DragEvent) => void;
    onDragLeave: (e: DragEvent) => void;
    onDrop: (e: DragEvent) => void;
  };
};

/**
 * A ROM dropped onto the start: in the app the webview reports the dropped paths (`onDragDropEvent`; it keeps HTML drops of files from
 * the page), in the browser the surface takes the HTML `drop`. One `.nds` path opens the wizard with it, a file without a path (browser)
 * opens it with the note to type the path; anything else is refused. While disabled (the wizard is open) a drop does nothing, and the
 * browser does not open a dropped file in place of the app either.
 */
export function useRomDrop(enabled: boolean, onRom: (r: StartWizardRequest) => void, onRefuse: (msg: string) => void): RomDrop {
  const [over, setOver] = useState(false);
  const latest = useRef({ onRom, onRefuse });
  latest.current = { onRom, onRefuse };
  // dragenter and dragleave come for every child the pointer crosses: the surface is left when they even out.
  const depth = useRef(0);

  const takePath = (path: string) => (isRomPath(path) ? latest.current.onRom({ rom: path }) : latest.current.onRefuse(REFUSED));

  useEffect(() => {
    if (!enabled) {
      setOver(false);
      depth.current = 0;
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled || !inTauri()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "enter" || payload.type === "over") setOver(true);
        else if (payload.type === "leave") setOver(false);
        else {
          setOver(false);
          if (payload.paths.length === 1) takePath(payload.paths[0]!);
          else latest.current.onRefuse(REFUSED);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not inside a Tauri webview */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [enabled]);

  const browser = !inTauri();
  const handlers: RomDrop["handlers"] = {
    onDragEnter: (e) => {
      if (!browser || !enabled || !carriesPath(e.dataTransfer)) return;
      // A drop target cancels dragenter as well as dragover.
      e.preventDefault();
      depth.current += 1;
      if (carriesFileDrag(e.dataTransfer)) setOver(true);
    },
    onDragOver: (e) => {
      if (!browser || !(enabled ? carriesPath : carriesFiles)(e.dataTransfer)) return;
      // Without this the browser would open a dropped file in place of the app.
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = enabled ? "copy" : "none";
    },
    onDragLeave: (e) => {
      if (!browser || !enabled || !carriesPath(e.dataTransfer)) return;
      depth.current = Math.max(0, depth.current - 1);
      if (depth.current === 0) setOver(false);
    },
    onDrop: (e) => {
      if (!browser || !(enabled ? carriesPath : carriesFiles)(e.dataTransfer)) return;
      e.preventDefault();
      depth.current = 0;
      setOver(false);
      if (!enabled) return;
      const source = pathFromDataTransfer(e.dataTransfer);
      if ("path" in source) takePath(source.path);
      else if ("file" in source) {
        if (isRomPath(source.file)) latest.current.onRom({ notice: NO_PATH });
        else latest.current.onRefuse(REFUSED);
      } else latest.current.onRefuse(REFUSED);
    },
  };
  return { over, handlers };
}
