import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createTextArchive, editTextArchive, errorText, searchText } from "../../api";
import type { DataRowDto, FigureDialog, HeaderSummary, MatrixSummary, MemberCounts, ProjectInfo, TextEdit, TextHit } from "../../api";
import type { EditBus } from "../../bus/editBus";
import type { ScriptFiles } from "../../data/useDataArea";
import { toast } from "../../ui/frame/toasts";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { dialogTitle, figureKey, figureTarget, speakersText } from "../dialog/dialogModel";
import type { DialogTarget } from "../dialog/dialogModel";
import type { NavTarget } from "../navigation";
import { buildPlaces, placeOf } from "../places";
import type { ProblemsStore } from "../problems/problems";
import { TERMS, id4 } from "../terms";
import { trainerLabel } from "../trainerScript";
import { TextAreaContext, TextDraft, useTextArea } from "./TextAreaContext";
import type { TextAreaModel, TextScope } from "./TextAreaContext";
import { LINE_LIMIT, SEARCH_LIMIT, archivePlaces, archiveRows, longestLine, usersByArchive } from "./textModel";
import type { TextTarget } from "./textModel";
import { HeaderDialogsCache, HeaderDialogsCacheContext, useCachedHeaderDialogs } from "./useHeaderDialogs";
import { useTextArchive } from "./useTextArchive";

/** What the text area is shown: the archive and message it has open (`message` null: none yet). */
export interface TextShown {
  archive: number | null;
  message: number | null;
}

/** What the text area needs from App. */
export interface TextAreaProps {
  /** The open project; another object means another project was opened: the area starts over and drops what it had not written. */
  project: ProjectInfo | null;
  /** The header table: each header's `text_archive_id` says which headers use an archive, its location name names the archive. */
  headers: HeaderSummary[];
  /** The member counts; `TextArchive` is the number of archives. */
  counts: MemberCounts;
  /** The archive and message a link asked for (a new object per request); null: none. */
  target: TextTarget | null;
  /** Changes whenever an archive may have moved behind the area (undo, redo, reload). */
  revision: number;
  /** The problems of the project: the overlong texts of the archives a save checked. */
  problems: ProblemsStore;
  /** Follows a link: a header that uses the archive leads to that header. */
  goTo: (target: NavTarget) => void;
  /** An archive was edited: refresh the dirty count. */
  onEdited: () => void;
  /**
   * Hears `textData` for every archive the area edited or created: names and descriptions of the data areas live in text archives.
   * The area itself also listens (its `revision`), so it reads its own edit once more.
   */
  bus: EditBus;
  /** An archive was created: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
  /** Hears which archive and message the area shows (the hinge's breadcrumb follows them). Must keep its identity. */
  onShown?: (shown: TextShown) => void;
  /** The depth the area shows (Erzählen, Bauen or Tüfteln): Bauen names the users of an archive places ("Ort"), Tüfteln headers. */
  depth: Depth;
  /** The matrices of the world, for the order of the headers in a place (outside first); omitted: by id. */
  matrices?: MatrixSummary[];
  /** Changes when the figures of a place or their dialogs may have moved (texts, scripts, events); omitted: `revision`. */
  dialogRevision?: number;
  /**
   * The Texte area is in front (omitted: it is). Behind another area (the deck stays mounted) the list's rows keep their last read: every
   * edit elsewhere moves `dialogRevision`, and an opened place would ask the session for each of its headers again.
   */
  inFront?: boolean;
  /** Opens the dialog editor of a figure (Erzählen). */
  onOpenDialog?: (target: DialogTarget) => void;
  /** Switches the depth (a link to a message no figure says opens Bauen). */
  onDepth?: (depth: Depth) => void;
  /** The trainer rows, which name a trainer figure by class and name (null while they are not read). */
  trainers?: readonly DataRowDto[] | null;
  /** Asks for the trainer rows (a place with a trainer figure shows). */
  needTrainers?: () => void;
  /** The script panel's documents: a split in the hand saves and reloads the figure's script file. */
  scriptFiles?: ScriptFiles;
}

/** What a link to a message tells when no figure of the places that use its archive says it. */
const NO_FIGURE_SAYS = "Keine Figur spricht diese Nachricht – in Bauen geöffnet";
/** What a link to an archive tells when no place uses the archive (a game-wide one). */
const NO_PLACE_USES = "Kein Ort nutzt dieses Archiv – in Bauen geöffnet";

/** „Trainer Wanderer Jens“; a trainer the rows do not (yet) have, or one without a name, is „Trainer“ (Erzählen names no numbers). */
function trainerTitle(rows: readonly DataRowDto[] | null | undefined, id: number): string {
  const label = trainerLabel(rows ?? null, id);
  return label === `Trainer ${id4(id)}` ? "Trainer" : `Trainer ${label}`;
}

/** Quiet time before the search over all archives asks the backend. */
const DEBOUNCE_MS = 300;

/**
 * The text area's model and state, for its list, view, hand and status parts below it (`useTextArea`). Memoised: it renders when
 * `context` changes (the owner keeps it, see `TextAreaHost`) or its children do, not for every render of the app.
 */
export const TextArea = memo(function TextArea({ context, children }: { context: TextAreaProps; children?: ReactNode }) {
  const { project, headers, counts, target, revision, problems, goTo, onShown, depth } = context;
  const [archiveIndex, setArchiveIndex] = useState<number | null>(null);
  const [message, setMessage] = useState(0);
  const [page, setPage] = useState(0);
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<TextScope>("all");
  const [hits, setHits] = useState<TextHit[] | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [editorFocused, setEditorFocused] = useState(false);
  const [draft] = useState(() => new TextDraft());
  const { archive: loaded, error, loading, reload } = useTextArchive(archiveIndex, revision);
  const narrating = !depthAtLeast(depth, "bauen");
  const dialogRevision = context.dialogRevision ?? revision;
  // The revision the list's rows read at: `dialogRevision` while the area is in front, the last one it had there while it is behind.
  const listRevisionRef = useRef(dialogRevision);
  if (context.inFront ?? true) listRevisionRef.current = dialogRevision;
  const dialogListRevision = listRevisionRef.current;
  // The figures of the places, per header and revision (a new cache per project), and the place and figure the dialogs show.
  const [cache, setCache] = useState(() => new HeaderDialogsCache());
  const [dialogHeader, setDialogHeader] = useState<number | null>(null);
  const [figureChosen, setFigureChosen] = useState<string | null>(null);

  // Another project: nothing of the old one stays open, typed or found. Reset while rendering, so no render shows the old
  // archive with the new project; commits of the old project are dropped by `run` (see `project` there).
  const [seenProject, setSeenProject] = useState(project);
  if (seenProject !== project) {
    setSeenProject(project);
    setArchiveIndex(null);
    setMessage(0);
    setPage(0);
    setQuery("");
    setHits(null);
    setSearchError(null);
    setCache(new HeaderDialogsCache());
    setDialogHeader(null);
    setFigureChosen(null);
  }

  // Nothing typed in the old project shows in the preview of the new one (the store tells its readers, hence not during the render).
  useEffect(() => draft.clear(), [project, draft]);

  // The callbacks of the latest render, for the commits that run later (a blur, a timer, an unmount).
  const latest = useRef(context);
  latest.current = context;
  const shown = useRef(archiveIndex);
  shown.current = archiveIndex;
  // Edits go to the backend one after the other, in the order they were made.
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  // `owner` is the project the edit was made in: an edit that outlives its project (a timer, an unmount flush, a queued job) is dropped.
  const run = useCallback((owner: ProjectInfo | null, index: number, edit: TextEdit): Promise<boolean> => {
    const job = chain.current.then(async () => {
      if (latest.current.project !== owner) return false;
      try {
        await editTextArchive(index, edit);
        latest.current.onEdited();
        latest.current.bus.emit({ topic: "textData", archive: index });
        return true;
      } catch (e) {
        latest.current.onError(e);
        return false;
      }
    });
    chain.current = job;
    return job;
  }, []);

  const users = useMemo(() => usersByArchive(headers), [headers]);
  const places = useMemo(() => archivePlaces(headers, users), [headers, users]);
  const archiveCount = counts.TextArchive;
  const rows = useMemo(() => archiveRows(archiveCount, users, places), [archiveCount, users, places]);
  const matrices = context.matrices;
  const worldPlaces = useMemo(() => buildPlaces(headers, matrices ?? []), [headers, matrices]);
  const { dialogs, error: dialogsError, loading: dialogsLoading } = useCachedHeaderDialogs(cache, dialogHeader, dialogRevision);
  const dialogFigure = useMemo(() => dialogs?.figures.find((f) => figureKey(f) === figureChosen) ?? null, [dialogs, figureChosen]);
  const dialogPlace = useMemo(() => (dialogHeader === null ? null : (placeOf(worldPlaces, dialogHeader) ?? null)), [worldPlaces, dialogHeader]);
  // For the link below, which reads the places and figures after it started.
  const dialogsNow = useRef({ narrating, places: worldPlaces, dialogRevision, cache });
  dialogsNow.current = { narrating, places: worldPlaces, dialogRevision, cache };

  const select = useCallback((index: number, messageIndex = 0) => {
    setArchiveIndex(index);
    setMessage(messageIndex);
    setPage(0);
  }, []);

  const selectDialogHeader = useCallback((header: number) => {
    setDialogHeader(header);
    setFigureChosen(null);
  }, []);

  // A link to a text archive opens it (and clears a search that would hide its messages). In Erzählen it shows the place and the figure
  // that says the message: among the headers that use the archive (in the order of the places) the first whose figures name it; without a
  // message the first of those headers. When no place uses the archive or no figure says the message, the link opens Bauen (with an info why).
  useEffect(() => {
    if (target === null) return;
    select(target.index, target.message ?? 0);
    setQuery("");
    const { narrating: narratingNow, places: placesNow, dialogRevision: revisionNow, cache: cacheNow } = dialogsNow.current;
    if (!narratingNow) return;
    const users = placesNow.flatMap((p) => p.headers.map((h) => h.header)).filter((h) => h.text_archive_id === target.index);
    const message = target.message;
    const toBauen = (why: string) => {
      latest.current.onDepth?.("bauen");
      toast.info(why);
    };
    if (users[0] === undefined) {
      toBauen(NO_PLACE_USES);
      return;
    }
    if (message === null) {
      selectDialogHeader(users[0].id);
      return;
    }
    let current = true;
    void (async () => {
      for (const h of users) {
        const found = await cacheNow.load(h.id, revisionNow);
        if (!current) return;
        const figure = found?.figures.find((f) => f.message_ids.includes(message));
        if (figure !== undefined) {
          setDialogHeader(h.id);
          setFigureChosen(figureKey(figure));
          return;
        }
      }
      toBauen(NO_FIGURE_SAYS);
    })();
    return () => {
      current = false;
    };
  }, [target, select, selectDialogHeader]);

  const effectiveScope: TextScope = scope === "archive" && archiveIndex !== null ? "archive" : "all";
  useEffect(() => {
    const q = query.trim();
    if (q === "" || effectiveScope === "archive") {
      setHits(null);
      setSearchError(null);
      return;
    }
    let current = true;
    const timer = setTimeout(() => {
      searchText(q, SEARCH_LIMIT).then(
        (found) => {
          if (!current) return;
          setHits(found);
          setSearchError(null);
        },
        (e) => current && setSearchError(errorText(e)),
      );
    }, DEBOUNCE_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [query, effectiveScope]);

  const messageCount = loaded?.messages.length ?? null;
  // Clamped once the messages are read; until then the one asked for.
  const selectedMessage = messageCount === null ? (archiveIndex === null ? null : message) : messageCount === 0 ? null : Math.min(message, messageCount - 1);
  const widths = useMemo(() => (loaded ? loaded.messages.map((m) => longestLine(m.text)) : []), [loaded]);

  useEffect(() => {
    onShown?.({ archive: archiveIndex, message: selectedMessage });
  }, [onShown, archiveIndex, selectedMessage]);

  const commit = useCallback(
    async (owner: ProjectInfo | null, index: number, messageIndex: number, text: string) => {
      const ok = await run(owner, index, { Set: { index: messageIndex, text } });
      if (ok && shown.current === index) void reload();
      return ok;
    },
    [run, reload],
  );

  const append = useCallback(async () => {
    const index = shown.current;
    if (index === null) return;
    if (!(await run(latest.current.project, index, "Append")) || shown.current !== index) return;
    const changed = await reload();
    if (changed) select(index, changed.messages.length - 1);
  }, [run, reload, select]);

  const removeLast = useCallback(async () => {
    const index = shown.current;
    if (index === null) return;
    if (!(await run(latest.current.project, index, "RemoveLast")) || shown.current !== index) return;
    const changed = await reload();
    if (changed) setMessage((m) => Math.min(m, changed.messages.length - 1));
  }, [run, reload]);

  const createArchive = useCallback(async () => {
    const owner = latest.current.project;
    try {
      const index = await createTextArchive();
      latest.current.bus.emit({ topic: "textData", archive: index });
      await latest.current.onCountsChanged();
      if (latest.current.project === owner) select(index);
    } catch (e) {
      latest.current.onError(e);
    }
  }, [select]);

  const selectFigure = useCallback((figure: FigureDialog | null) => setFigureChosen(figure === null ? null : figureKey(figure)), []);
  const openDialogEditor = useCallback(
    (figure: FigureDialog) => {
      if (dialogs !== null) latest.current.onOpenDialog?.(figureTarget(dialogs, figure));
    },
    [dialogs],
  );
  const trainers = context.trainers;
  const figureTitle = useCallback(
    (figure: FigureDialog) => (figure.source.kind === "trainer" ? trainerTitle(trainers, figure.source.id) : dialogTitle(figure.kind, figure.index)),
    [trainers],
  );
  const needTrainers = useCallback(() => latest.current.needTrainers?.(), []);
  const dialogChanged = useCallback(() => {
    latest.current.onEdited();
    latest.current.bus.emit({ topic: "textData", archive: null });
  }, []);
  const onError = useCallback((e: unknown) => latest.current.onError(e), []);
  const scriptFiles = context.scriptFiles;
  const dialogsFound = useMemo(() => ({ dialogs, error: dialogsError, loading: dialogsLoading }), [dialogs, dialogsError, dialogsLoading]);

  const model = useMemo<TextAreaModel>(
    () => ({
      archive: archiveIndex,
      message: selectedMessage,
      page,
      query,
      scope,
      select,
      setPage,
      setQuery,
      setScope,
      project,
      headers,
      archiveCount,
      rows,
      users,
      loaded,
      error,
      loading,
      widths,
      effectiveScope,
      hits,
      searchError,
      problems,
      draft,
      editorFocused,
      setEditorFocused,
      commit,
      append: () => void append(),
      removeLast: () => void removeLast(),
      createArchive: () => void createArchive(),
      goTo,
      depth,
      narrating,
      places: worldPlaces,
      dialogHeader,
      dialogPlace,
      dialogs: dialogsFound,
      dialogFigure,
      dialogRevision,
      dialogListRevision,
      selectDialogHeader,
      selectFigure,
      openDialogEditor,
      figureTitle,
      needTrainers,
      dialogChanged,
      onError,
      scriptFiles,
    }),
    [
      archiveIndex, selectedMessage, page, query, scope, select, project, headers, archiveCount, rows, users, loaded, error, loading, widths, effectiveScope, hits, searchError, problems, draft, editorFocused, commit, append,
      removeLast, createArchive, goTo, depth, narrating, worldPlaces, dialogHeader, dialogPlace, dialogsFound, dialogFigure, dialogRevision, dialogListRevision, selectDialogHeader, selectFigure, openDialogEditor, figureTitle, needTrainers,
      dialogChanged, onError, scriptFiles,
    ],
  );
  return (
    <TextAreaContext.Provider value={model}>
      <HeaderDialogsCacheContext.Provider value={cache}>{children}</HeaderDialogsCacheContext.Provider>
    </TextAreaContext.Provider>
  );
});

/** How many messages of the open archive have a line over the limit. */
export const overlongCount = (widths: readonly number[]) => widths.filter((w) => w > LINE_LIMIT).length;

/**
 * The middle of the status line while the texts show: "Archiv 0412, 34 Nachrichten, 1 zu lang"; empty without an open archive. In Erzählen
 * the place, how many figures speak there and the chosen figure ("Zweiblattdorf, 2 Figuren sprechen, Objekt 3").
 */
export function TextStatus() {
  const { archive, loaded, widths, narrating, dialogPlace, dialogs, dialogFigure, figureTitle } = useTextArea();
  if (narrating) {
    if (dialogPlace === null) return null;
    const parts = [dialogPlace.name];
    if (dialogs.dialogs !== null) parts.push(speakersText(dialogs.dialogs.figures));
    if (dialogFigure !== null) parts.push(figureTitle(dialogFigure));
    return <>{parts.join(", ")}</>;
  }
  if (archive === null) return null;
  const parts = [`Archiv ${id4(archive)}`];
  if (loaded) {
    const n = loaded.messages.length;
    parts.push(`${n} ${n === 1 ? TERMS.message.label : TERMS.message.plural}`);
    const long = overlongCount(widths);
    if (long > 0) parts.push(`${long} zu lang`);
  }
  return <>{parts.join(", ")}</>;
}

/** The left of the status line while the texts show: the keys of the message field while it has the focus. */
export function TextHints() {
  const { editorFocused } = useTextArea();
  return editorFocused ? <>↵ neue Zeile, ⇧↵ neue Seite</> : null;
}
