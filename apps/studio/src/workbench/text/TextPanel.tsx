import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createTextArchive, editTextArchive, errorText, searchText, validateText } from "../../api";
import type { HeaderSummary, MemberCounts, ProblemDto, TextEdit, TextHit } from "../../api";
import type { NavTarget } from "../navigation";
import { TERMS, id4 } from "../terms";
import { archiveLabel, archiveRows, filterArchives, hitLabel, segments, SEARCH_LIMIT, toTextForm, usersByArchive, usersLabel, visibleLine } from "./textModel";
import type { ArchiveRow, SegmentKind, TextTarget } from "./textModel";
import { useTextArchive } from "./useTextArchive";

/** What the text panel needs from App. */
export interface TextPanelContext {
  /** The header table: each header's `text_archive_id` says which headers use an archive. */
  headers: HeaderSummary[];
  /** The member counts; `TextArchive` is the number of archives. */
  counts: MemberCounts;
  /** The archive and message a link asked for (a new object per request); null: none. */
  target: TextTarget | null;
  /** Changes whenever an archive may have moved behind the panel (undo, redo, reload). */
  revision: number;
  /** Follows a link: a header number above the messages leads to that header. */
  goTo: (target: NavTarget) => void;
  /** An archive was edited: refresh the dirty count. */
  onEdited: () => void;
  /** An archive was created: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/** Quiet time after the last keystroke before a text is committed (a blur commits at once). */
const COMMIT_MS = 500;
/** Quiet time before the search and the length check ask the backend. */
const DEBOUNCE_MS = 300;

const mono: CSSProperties = { fontFamily: "ui-monospace, monospace" };
const ellipsis: CSSProperties = { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" };
const rowButton: CSSProperties = { all: "unset", boxSizing: "border-box", display: "block", width: "100%", cursor: "pointer", lineHeight: 1.6, padding: "0 8px", ...ellipsis };
const selectedRow: CSSProperties = { background: "#eff6ff", boxShadow: "inset 3px 0 0 #2563eb", fontWeight: 700 };
const heading: CSSProperties = { margin: 0, padding: "4px 8px", fontSize: 11, color: "#6b7280", textTransform: "uppercase", letterSpacing: 0.5 };
const note: CSSProperties = { margin: 0, padding: "8px 12px", color: "#6b7280" };
const linkButton: CSSProperties = { all: "unset", cursor: "pointer", color: "#2563eb", textDecoration: "underline" };

/** What each kind of segment looks like in the preview under the field. */
const SEGMENT_STYLE: Record<SegmentKind, CSSProperties> = {
  text: {},
  newline: { color: "#2563eb", fontWeight: 700 },
  scroll: { color: "#b45309", fontWeight: 700 },
  clear: { color: "#0f766e", fontWeight: 700 },
  code: { color: "#7c3aed", background: "#f5f3ff", borderRadius: 3, padding: "0 2px" },
};
const SEGMENT_SHOWN: Partial<Record<SegmentKind, string>> = { newline: "⏎", scroll: "▼", clear: "▽" };

/** One archive of the list. Memoised: a selection or filter change renders the rows whose props changed, not all of them. */
const ArchiveItem = memo(function ArchiveItem({ row, selected, count, onSelect }: { row: ArchiveRow; selected: boolean; count: number | null; onSelect: (index: number) => void }) {
  const users = usersLabel(row.users);
  // Only the open archive's message count is known: reading all of them to list it would decode every archive.
  const parts = [archiveLabel(row.index), count !== null ? `${count} ${count === 1 ? TERMS.message.label : TERMS.message.plural}` : null, users || null];
  return (
    <li>
      <button data-archive={row.index} onClick={() => onSelect(row.index)} aria-current={selected ? "true" : undefined} style={{ ...rowButton, ...(selected ? selectedRow : {}) }}>
        {parts.filter((p) => p !== null).join(" · ")}
      </button>
    </li>
  );
});

/** One message of the open archive: its number and its text on one line, break codes made visible. */
const MessageItem = memo(function MessageItem({ index, text, selected, onSelect }: { index: number; text: string; selected: boolean; onSelect: (index: number) => void }) {
  return (
    <li>
      <button data-message={index} onClick={() => onSelect(index)} aria-current={selected ? "true" : undefined} style={{ ...rowButton, display: "grid", gridTemplateColumns: "44px minmax(0, 1fr)", ...(selected ? selectedRow : {}) }}>
        <span style={{ ...mono, color: "#6b7280" }}>{index}</span>
        <span style={ellipsis}>{visibleLine(text) || "(leer)"}</span>
      </button>
    </li>
  );
});

/**
 * The field of one message. It keeps the typed text and commits it on blur, after `COMMIT_MS` of
 * rest and when it goes away (a switch to another message or archive): `onCommit` resolves to
 * whether the backend took it, and a refused text is dropped for the stored one. While the archive
 * is read again (after a commit, undo, redo) the field follows the stored text unless there is typing the backend does not have yet.
 */
function MessageEditor({ value, onCommit }: { value: string; onCommit: (text: string) => Promise<boolean> }) {
  const [draft, setDraft] = useState(value);
  const draftRef = useRef(draft);
  // The text the backend has, or is about to have: what the draft is compared with.
  const sent = useRef(value);
  const latest = useRef({ value, onCommit });
  latest.current = { value, onCommit };
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);
  const field = useRef<HTMLTextAreaElement>(null);
  const caret = useRef<number | null>(null);
  const [problems, setProblems] = useState<ProblemDto[]>([]);

  useEffect(() => {
    if (draftRef.current === sent.current) {
      draftRef.current = value;
      setDraft(value);
    }
    sent.current = value;
  }, [value]);

  const flush = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
    const text = draftRef.current;
    if (text === sent.current) return;
    sent.current = text;
    void latest.current.onCommit(text).then((ok) => {
      if (ok || !mounted.current) return;
      // Refused: the field shows what is stored again.
      sent.current = latest.current.value;
      draftRef.current = sent.current;
      setDraft(sent.current);
    });
  }, []);

  useEffect(
    () => () => {
      mounted.current = false;
      flush();
    },
    [flush],
  );

  // A line break typed or pasted is written as `\n`, and the caret stays where it was typed.
  useLayoutEffect(() => {
    if (caret.current !== null && field.current) field.current.setSelectionRange(caret.current, caret.current);
    caret.current = null;
  }, [draft]);

  function change(raw: string, at: number) {
    const text = toTextForm(raw);
    caret.current = toTextForm(raw.slice(0, at)).length;
    draftRef.current = text;
    setDraft(text);
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = setTimeout(flush, COMMIT_MS);
  }

  // The length check, for the stored text on selection and for the draft while typing.
  useEffect(() => {
    let current = true;
    const check = setTimeout(() => {
      validateText(draft).then(
        (found) => current && setProblems(found),
        () => current && setProblems([]),
      );
    }, DEBOUNCE_MS);
    return () => {
      current = false;
      clearTimeout(check);
    };
  }, [draft]);

  const parts = useMemo(() => segments(draft), [draft]);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, padding: "8px 12px", borderTop: "1px solid #e5e7eb" }}>
      <textarea
        ref={field}
        value={draft}
        aria-label="Text der Nachricht"
        rows={5}
        spellCheck={false}
        onChange={(e) => change(e.target.value, e.target.selectionStart)}
        onBlur={flush}
        onKeyDown={(e) => {
          // Cmd+S saves the project (the app's handler, further up): what is typed has to be written first.
          if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") flush();
        }}
        style={{ ...mono, width: "100%", boxSizing: "border-box", padding: 6, resize: "vertical" }}
      />
      {parts.length > 0 && (
        <div aria-label="Vorschau" style={{ ...mono, fontSize: 12, wordBreak: "break-word" }}>
          {parts.map((s, i) => (
            <span key={i} data-segment={s.kind} title={s.kind === "text" ? undefined : s.text} style={SEGMENT_STYLE[s.kind]}>
              {SEGMENT_SHOWN[s.kind] ?? s.text}
            </span>
          ))}
        </div>
      )}
      {problems.length > 0 && (
        <ul aria-label="Warnungen" style={{ margin: 0, paddingLeft: 18, color: "#b45309", fontSize: 12 }}>
          {problems.map((p, i) => (
            <li key={i}>
              Zeile {p.line}: {p.message}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * Memoised: renders when `context` changes (the owner keeps it, see `TextTab`), not for every
 * render of the panel host. Archive list and search on the left, the messages of the chosen
 * archive and the field of the chosen message on the right.
 */
export const TextPanel = memo(function TextPanel({ context }: { context: TextPanelContext }) {
  const { headers, counts, target, revision, goTo } = context;
  const [archiveIndex, setArchiveIndex] = useState<number | null>(null);
  const [message, setMessage] = useState(0);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<TextHit[] | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const { archive, error, loading, reload } = useTextArchive(archiveIndex, revision);
  const list = useRef<HTMLUListElement>(null);

  // The callbacks of the latest render, for the commits that run later (a blur, a timer, an unmount).
  const latest = useRef(context);
  latest.current = context;
  const shown = useRef(archiveIndex);
  shown.current = archiveIndex;
  // Edits go to the backend one after the other, in the order they were made.
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const run = useCallback((index: number, edit: TextEdit): Promise<boolean> => {
    const job = chain.current.then(async () => {
      try {
        await editTextArchive(index, edit);
        latest.current.onEdited();
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
  const rows = useMemo(() => archiveRows(counts.TextArchive, users), [counts.TextArchive, users]);
  const visible = useMemo(() => filterArchives(rows, query), [rows, query]);

  const select = useCallback((index: number, messageIndex = 0) => {
    setArchiveIndex(index);
    setMessage(messageIndex);
  }, []);
  const selectArchive = useCallback((index: number) => select(index), [select]);

  // A link to a text archive opens it (and clears a search that would hide it).
  useEffect(() => {
    if (target === null) return;
    select(target.index, target.message ?? 0);
    setQuery("");
  }, [target, select]);

  // The archive that opens is scrolled into view.
  useEffect(() => {
    if (archiveIndex !== null) list.current?.querySelector(`[data-archive="${archiveIndex}"]`)?.scrollIntoView({ block: "nearest" });
  }, [archiveIndex, rows.length]);

  useEffect(() => {
    const q = query.trim();
    if (q === "") {
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
  }, [query]);

  const messages = archive?.messages ?? [];
  const selectedMessage = messages.length === 0 ? null : Math.min(message, messages.length - 1);

  const commit = useCallback(
    async (index: number, messageIndex: number, text: string) => {
      const ok = await run(index, { Set: { index: messageIndex, text } });
      if (ok && shown.current === index) void reload();
      return ok;
    },
    [run, reload],
  );

  async function append() {
    if (archiveIndex === null) return;
    if (!(await run(archiveIndex, "Append")) || shown.current !== archiveIndex) return;
    const changed = await reload();
    if (changed) setMessage(changed.messages.length - 1);
  }

  async function removeLast() {
    if (archiveIndex === null) return;
    if (!(await run(archiveIndex, "RemoveLast")) || shown.current !== archiveIndex) return;
    const changed = await reload();
    if (changed) setMessage((m) => Math.min(m, changed.messages.length - 1));
  }

  async function createArchive() {
    try {
      const index = await createTextArchive();
      await latest.current.onCountsChanged();
      select(index);
    } catch (e) {
      latest.current.onError(e);
    }
  }

  const noQuery = query.trim() === "";
  const opened = archiveIndex === null ? [] : (users.get(archiveIndex) ?? []);
  return (
    <div style={{ display: "flex", height: "100%", boxSizing: "border-box", fontFamily: "system-ui, sans-serif", fontSize: 13 }}>
      <div style={{ display: "flex", flexDirection: "column", width: 280, flexShrink: 0, borderRight: "1px solid #e5e7eb" }}>
        <div style={{ padding: "8px 8px 4px" }}>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Suchen nach Text oder Nummer"
            aria-label="Texte suchen"
            style={{ width: "100%", boxSizing: "border-box", padding: "4px 8px" }}
          />
        </div>
        {!noQuery && (
          <div style={{ maxHeight: "45%", overflowY: "auto", flexShrink: 0 }}>
            <p style={heading}>{searchError ?? (hits === null ? "Suche …" : hits.length === 0 ? "Keine Treffer" : `${hits.length} Treffer`)}</p>
            <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
              {(hits ?? []).map((h) => (
                <li key={`${h.archive}:${h.index}`}>
                  <button data-hit={`${h.archive}:${h.index}`} onClick={() => select(h.archive, h.index)} title={hitLabel(h)} style={rowButton}>
                    {hitLabel(h)}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
        <ul ref={list} style={{ flex: 1, minHeight: 0, overflowY: "auto", listStyle: "none", margin: 0, padding: "0 0 8px" }}>
          {visible.map((row) => (
            <ArchiveItem key={row.index} row={row} selected={row.index === archiveIndex} count={row.index === archiveIndex && archive ? archive.messages.length : null} onSelect={selectArchive} />
          ))}
        </ul>
        <div style={{ padding: 8, borderTop: "1px solid #e5e7eb" }}>
          <button onClick={() => void createArchive()} disabled={counts.TextArchive === undefined} title="Hängt ein Textarchiv mit einer Nachricht an">
            Neues Archiv
          </button>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", flex: 1, minWidth: 0 }}>
        {archiveIndex === null ? (
          <p style={note}>Textarchiv links wählen oder nach einem Text suchen.</p>
        ) : (
          <>
            <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, padding: "8px 12px" }}>
              <strong>{archiveLabel(archiveIndex)}</strong>
              {archive && (
                <span>
                  {archive.messages.length} {archive.messages.length === 1 ? TERMS.message.label : TERMS.message.plural}
                </span>
              )}
              {opened.length > 0 && (
                <span>
                  {TERMS.header.label}{" "}
                  {opened.map((id, i) => (
                    <span key={id}>
                      {i > 0 && ", "}
                      <button data-header={id} onClick={() => goTo({ kind: "header", id })} title="Header im Welt-Explorer zeigen" style={linkButton}>
                        {id4(id)}
                      </button>
                    </span>
                  ))}
                </span>
              )}
              <span style={{ flex: 1 }} />
              <button onClick={() => void append()} disabled={archive === null}>
                Nachricht anfügen
              </button>
              <button onClick={() => void removeLast()} disabled={archive === null || messages.length <= 1}>
                Letzte entfernen
              </button>
            </div>
            {error !== null && (
              <p role="alert" style={{ ...note, color: "#b91c1c" }}>
                {error}
              </p>
            )}
            {loading && <p style={note}>Lade …</p>}
            {archive && (
              <>
                <ul style={{ flex: 1, minHeight: 0, overflowY: "auto", listStyle: "none", margin: 0, padding: 0 }}>
                  {messages.map((m, i) => (
                    <MessageItem key={i} index={i} text={m.text} selected={i === selectedMessage} onSelect={setMessage} />
                  ))}
                </ul>
                {selectedMessage !== null && (
                  <MessageEditor
                    key={`${archiveIndex}:${selectedMessage}`}
                    value={messages[selectedMessage].text}
                    onCommit={(text) => commit(archiveIndex, selectedMessage, text)}
                  />
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
});
