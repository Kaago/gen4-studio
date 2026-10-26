import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { addScript, adoptScripts, readScript, saveLevelScript, saveScript, scriptsStatus } from "../../api";
import type { AdoptProgressDto, AdoptReportDto, CmdError, LevelScriptDoc, ProjectInfo, SaveScriptDto, ScriptsStatusDto } from "../../api";
import type { EditBus } from "../../bus/editBus";
import { TERMS, id4 } from "../terms";
import { scriptPath } from "../problems/problems";
import type { ProblemsStore } from "../problems/problems";
import { ScriptDocuments, lineOffset } from "./documents";

/** What the rest of the app sees of the open scripts: the project session saves them before a build and counts them as unsaved. */
export interface ScriptBridge {
  /** The files whose text differs from what is on disk. */
  dirtyFiles(): number[];
  /** Saves the document the way Cmd+S does; rejects when the save fails or is refused. */
  save(file: number): Promise<void>;
}

export const NO_SCRIPTS: ScriptBridge = { dirtyFiles: () => [], save: async () => {} };

/** The script state the panels read (`WorkbenchData.scripts`). One object per change: `revision` says when something moved. */
export interface ScriptsApi {
  /** Null until the project's status arrived. */
  status: ScriptsStatusDto | null;
  /** The open documents. Mutable: the editor writes `text`, `state` and `cursor` into them; read them again when `revision` changes. */
  documents: ScriptDocuments;
  /** The shown document's file; null: none open. */
  active: number | null;
  /** Changes when a document opened, closed, was saved, turned dirty or clean, or got other text from outside. */
  revision: number;
  /** Opens the file (or shows it when open) and moves the cursor to the 1-based `line`. Failures are reported through `onError`. */
  open(file: number, line?: number): Promise<void>;
  /** Writes the document, compiles, hands the problems to the problems store and tells the text panel about archives the compile touched. Rejects like `ScriptBridge.save`. */
  save(file: number): Promise<void>;
  /** Closes the document without asking; the panel asks first when it is dirty. */
  close(file: number): void;
  /** Decompiles the project into rotom sources; the status says `adopted` afterwards. Rejects with the backend error. */
  adopt(onProgress?: (progress: AdoptProgressDto) => void): Promise<AdoptReportDto>;
  /** Appends an empty script to the file (saving it first when it is dirty) and shows it. Resolves to the new script's number; a failure goes to `onError` and resolves to null. */
  add(file: number): Promise<number | null>;
  /** Saves the document when it is open with unsaved text (before the backend writes the file itself); rejects like `save`. */
  saveIfDirty(file: number): Promise<void>;
  /**
   * The backend wrote the file itself (a new trainer's battle slots, an agent's save): an open document shows the text on disk, the language server
   * hears of it. Text typed in the meantime stays, dirty, and `onError` says that saving it overwrites what the backend wrote.
   */
  reload(file: number): Promise<void>;
  /**
   * The level-script form saved `doc` as file `file` (not the editor): the problems and archives are handled like those of a
   * save, and an open document without unsaved text shows the new JSON. Ignored when `project` is not the open one any more.
   */
  onLevelScriptSaved(project: ProjectInfo | null, file: number, doc: LevelScriptDoc, dto: SaveScriptDto): void;
  /** The editor changed the text of the document: renders again when that makes it dirty or clean. */
  onChange(file: number): void;
}

export interface ScriptsController {
  api: ScriptsApi;
  bridge: ScriptBridge;
  /** Open documents with unsaved text. */
  dirtyCount: number;
}

interface ScriptsParams {
  /** The open project; another object means another one was opened: documents and status start over. */
  info: ProjectInfo | null;
  bus: EditBus;
  problems: ProblemsStore;
  onError: (e: unknown) => void;
}

const lf = (text: string) => text.replace(/\r\n?/g, "\n");

/** A refusal in the shape of a backend error, so `errorText` and the message bar treat it like one. */
const refusal = (message: string): CmdError => ({ code: "error", message, asset: null });

/** The text a level script is edited as; `parseLevelScript` is its way back. */
const levelScriptText = (doc: LevelScriptDoc) => `${JSON.stringify(doc, null, 2)}\n`;

function parseLevelScript(file: number, text: string): LevelScriptDoc {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) return parsed as LevelScriptDoc;
  } catch (e) {
    throw refusal(`${TERMS.levelScript.label} ${id4(file)}: ungültiges JSON (${e instanceof Error ? e.message : String(e)})`);
  }
  throw refusal(`${TERMS.levelScript.label} ${id4(file)}: ungültiges JSON (ein Objekt wird erwartet)`);
}

/**
 * The project's scripts: whether rotom has adopted them, the open documents, and the commands on them. Lives in App,
 * not in the Script panel, because the toolbar counts unsaved scripts and a build saves them first. It does not touch the
 * editor or the language server (those, with CodeMirror, load with the panel).
 */
export function useScripts({ info, bus, problems, onError }: ScriptsParams): ScriptsController {
  // Another project: nothing of the old one stays open. Reset while rendering, so no render pairs the old documents with the new project.
  // `saving`: the saves in flight of this project, so a second one (Cmd+S reaches the editor and the app) joins the first.
  // It belongs to the project: a save of the same file number in the next project must not join the old one.
  const fresh = () => ({ info, documents: new ScriptDocuments(), saving: new Map<number, Promise<void>>() });
  const [owner, setOwner] = useState(fresh);
  const [status, setStatus] = useState<ScriptsStatusDto | null>(null);
  const [active, setActive] = useState<number | null>(null);
  let owned = owner;
  if (owner.info !== info) {
    owned = fresh();
    setOwner(owned);
    setStatus(null);
    setActive(null);
  }
  const { documents } = owned;
  const [revision, bump] = useReducer((n: number) => n + 1, 0);

  const latest = useRef({ info, documents, saving: owned.saving, problems, bus, onError });
  latest.current = { info, documents, saving: owned.saving, problems, bus, onError };
  const dirtyKey = useRef("");

  useEffect(() => {
    if (!info) return;
    let current = true;
    scriptsStatus().then(
      (next) => current && setStatus(next),
      (e: unknown) => current && latest.current.onError(e),
    );
    return () => {
      current = false;
    };
  }, [info]);

  const open = useCallback(async (file: number, line?: number) => {
    const docs = latest.current.documents;
    try {
      let doc = docs.get(file);
      if (!doc) {
        const source = await readScript(file);
        // Another project was opened while the file was read.
        if (latest.current.documents !== docs) return;
        if (source.kind === "binary") throw refusal(`${TERMS.scriptFile.label} ${id4(file)} ist binär (${source.size} Byte) und lässt sich nicht bearbeiten`);
        // rotom writes LF; the documents compare LF with LF.
        doc = docs.open(file, source.kind, lf(source.kind === "script" ? source.text : levelScriptText(source.doc)));
      }
      if (line !== undefined) doc.cursor = lineOffset(doc.text, line);
      setActive(file);
      bump();
    } catch (e) {
      latest.current.onError(e);
    }
  }, []);

  const save = useCallback((file: number): Promise<void> => {
    const { documents: docs, saving } = latest.current;
    const doc = docs.get(file);
    if (!doc) return Promise.resolve();
    const running = saving.get(file);
    // A joined save must not leave newer text behind: what was typed after the running save began is saved right after it.
    if (running) return running.then(() => (lf(doc.text) !== doc.savedText ? save(file) : undefined));
    const job = (async () => {
      const sent = lf(doc.text);
      const dto: SaveScriptDto = doc.kind === "script" ? await saveScript(file, sent) : await saveLevelScript(file, parseLevelScript(file, sent));
      // Another project was opened while the backend wrote: its problems and archives are not this project's.
      if (latest.current.documents !== docs) return;
      // Typing during the save stays dirty: only what was sent is on disk now.
      doc.savedText = sent;
      latest.current.problems.applyReport(dto.problems, { files: [scriptPath(file, doc.kind)] });
      for (const archive of dto.touched_archives) latest.current.bus.emit({ topic: "textData", archive });
      // The checks read the sources from the disk: they look again.
      latest.current.bus.emit({ topic: "edited" });
      bump();
    })().finally(() => saving.delete(file));
    saving.set(file, job);
    return job;
  }, []);

  const close = useCallback((file: number) => {
    const { documents: docs } = latest.current;
    docs.close(file);
    setActive((shown) => {
      const rest = docs.list();
      return shown === file ? (rest[rest.length - 1]?.file ?? null) : shown;
    });
    bump();
  }, []);

  const adopt = useCallback(async (onProgress?: (progress: AdoptProgressDto) => void) => {
    const report = await adoptScripts((p) => onProgress?.(p));
    const docs = latest.current.documents;
    // rotom wrote its config: a running server (none yet, normally) reads the project state again.
    latest.current.bus.emit({ topic: "lspFilesChanged", changes: [{ path: "rotom.toml", type: 1 }] });
    // The checks read the new sources (the message check runs from now on).
    latest.current.bus.emit({ topic: "edited" });
    try {
      const next = await scriptsStatus();
      if (latest.current.documents === docs) setStatus(next);
    } catch {
      // The adoption happened; a status that cannot be read is not worth failing it for.
      if (latest.current.documents === docs) setStatus((previous) => ({ file_count: previous?.file_count ?? 0, adopted: true }));
    }
    return report;
  }, []);

  const add = useCallback(
    async (file: number): Promise<number | null> => {
      const docs = latest.current.documents;
      try {
        const doc = docs.get(file);
        if (doc && lf(doc.text) !== doc.savedText) await save(file);
        const n = await addScript(file);
        // Written: the checks look again.
        latest.current.bus.emit({ topic: "edited" });
        // The source on disk has the new script now: show it, with the cursor at its end.
        const source = await readScript(file);
        if (latest.current.documents !== docs) return null;
        if (source.kind === "script") {
          const text = lf(source.text);
          const shown = docs.open(file, "script", text);
          shown.text = text;
          shown.savedText = text;
          shown.cursor = text.length;
          setActive(file);
          bump();
        }
        // The compile that made the script may have appended to text archives.
        latest.current.bus.emit({ topic: "textData", archive: null });
        return n;
      } catch (e) {
        latest.current.onError(e);
        return null;
      }
    },
    [save],
  );

  const saveIfDirty = useCallback(
    async (file: number): Promise<void> => {
      const doc = latest.current.documents.get(file);
      if (doc && lf(doc.text) !== doc.savedText) await save(file);
    },
    [save],
  );

  const reload = useCallback(async (file: number): Promise<void> => {
    const { documents: docs, bus } = latest.current;
    bus.emit({ topic: "lspFilesChanged", changes: [{ path: scriptPath(file, "script"), type: 2 }] });
    const doc = docs.get(file);
    if (!doc || doc.kind !== "script") return;
    try {
      const source = await readScript(file);
      if (latest.current.documents !== docs || source.kind !== "script") return;
      const text = lf(source.text);
      if (lf(doc.text) === doc.savedText) doc.text = text;
      else {
        latest.current.onError(
          refusal(`${TERMS.scriptFile.label} ${id4(file)} wurde außerhalb des Editors geändert; deine ungespeicherten Änderungen überschreiben das beim Speichern`),
        );
      }
      doc.savedText = text;
      bump();
    } catch (e) {
      latest.current.onError(e);
    }
  }, []);

  const onLevelScriptSaved = useCallback((project: ProjectInfo | null, file: number, doc: LevelScriptDoc, dto: SaveScriptDto) => {
    const { info: shown, documents: docs, problems, bus } = latest.current;
    // Another project was opened while the form's save ran: its problems and archives are not this project's.
    if (shown !== project) return;
    problems.applyReport(dto.problems, { files: [scriptPath(file, "levelScript")] });
    for (const archive of dto.touched_archives) bus.emit({ topic: "textData", archive });
    bus.emit({ topic: "edited" });
    const open = docs.get(file);
    // A document with unsaved text keeps it: the editor's own save decides then.
    if (open && open.kind === "levelScript" && lf(open.text) === open.savedText) {
      const text = lf(levelScriptText(doc));
      open.text = text;
      open.savedText = text;
      bump();
    }
  }, []);

  const onChange = useCallback(() => {
    const key = latest.current.documents.dirty().join(",");
    if (key === dirtyKey.current) return;
    dirtyKey.current = key;
    bump();
  }, []);

  // A save or a close changes the dirty set without going through `onChange`.
  dirtyKey.current = documents.dirty().join(",");
  const dirtyCount = documents.dirty().length;

  const api = useMemo<ScriptsApi>(
    () => ({ status, documents, active, revision, open, save, close, adopt, add, saveIfDirty, reload, onLevelScriptSaved, onChange }),
    [status, documents, active, revision, open, save, close, adopt, add, saveIfDirty, reload, onLevelScriptSaved, onChange],
  );
  const bridge = useMemo<ScriptBridge>(() => ({ dirtyFiles: () => latest.current.documents.dirty(), save }), [save]);
  return { api, bridge, dirtyCount };
}
