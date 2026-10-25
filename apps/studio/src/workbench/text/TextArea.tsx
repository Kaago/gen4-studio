import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createTextArchive, editTextArchive, errorText, searchText } from "../../api";
import type { HeaderSummary, MemberCounts, ProjectInfo, TextEdit, TextHit } from "../../api";
import type { EditBus } from "../../bus/editBus";
import type { Depth } from "../../ui/frame/shellState";
import type { NavTarget } from "../navigation";
import type { ProblemsStore } from "../problems/problems";
import { TERMS, id4 } from "../terms";
import { TextAreaContext, TextDraft, useTextArea } from "./TextAreaContext";
import type { TextAreaModel, TextScope } from "./TextAreaContext";
import { LINE_LIMIT, SEARCH_LIMIT, archivePlaces, archiveRows, longestLine, usersByArchive } from "./textModel";
import type { TextTarget } from "./textModel";
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
  /** Bauen names the users of an archive places ("Ort"), Tüfteln headers. */
  depth: Depth;
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

  const select = useCallback((index: number, messageIndex = 0) => {
    setArchiveIndex(index);
    setMessage(messageIndex);
    setPage(0);
  }, []);

  // A link to a text archive opens it (and clears a search that would hide its messages).
  useEffect(() => {
    if (target === null) return;
    select(target.index, target.message ?? 0);
    setQuery("");
  }, [target, select]);

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
    }),
    [archiveIndex, selectedMessage, page, query, scope, select, project, headers, archiveCount, rows, users, loaded, error, loading, widths, effectiveScope, hits, searchError, problems, draft, editorFocused, commit, append, removeLast, createArchive, goTo, depth],
  );
  return <TextAreaContext.Provider value={model}>{children}</TextAreaContext.Provider>;
});

/** How many messages of the open archive have a line over the limit. */
export const overlongCount = (widths: readonly number[]) => widths.filter((w) => w > LINE_LIMIT).length;

/** The middle of the status line while the texts show: "Archiv 0412, 34 Nachrichten, 1 zu lang"; empty without an open archive. */
export function TextStatus() {
  const { archive, loaded, widths } = useTextArea();
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
