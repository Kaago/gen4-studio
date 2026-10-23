import { memo, useCallback, useMemo, useRef } from "react";
import type { CSSProperties, ReactNode, RefObject } from "react";
import type { TextHit } from "../../api";
import { Badge, Button, EmptyState, Icon, Notice, PanelHead, Reference, Segmented, TextField } from "../../ui/primitives";
import { id4 } from "../terms";
import { MessageEditor } from "./MessageEditor";
import { draftKey, useTextArea } from "./TextAreaContext";
import type { TextScope } from "./TextAreaContext";
import { TextTable } from "./TextTable";
import { LINE_LIMIT, id3, messageLabel, segments } from "./textModel";
import styles from "./TextHand.module.css";

const MESSAGE_COLUMNS = [{ title: "Nr." }, { title: "Text" }, { title: "Längste Zeile" }] as const;
const HIT_COLUMNS = [{ title: "Archiv" }, { title: "Nr." }, { title: "Text" }] as const;
const SCOPES = [
  { value: "archive", label: "Dieses Archiv" },
  { value: "all", label: "Alle" },
] as const;
const BREAK_SHOWN = { newline: "⏎", clear: "▼", scroll: "▽" } as const;

/** A message on one line: its text, the break codes as symbols, the other codes as badges. */
const MessageText = memo(function MessageText({ text }: { text: string }) {
  return (
    <span className={styles.text}>
      {segments(text).map((s, i) =>
        s.kind === "code" ? (
          <Badge key={i} kind="code">
            {s.text}
          </Badge>
        ) : s.kind === "text" ? (
          <span key={i}>{s.text.replace(/\\\\/g, "\\")}</span>
        ) : (
          <span key={i} className={styles.break} title={s.text}>
            {BREAK_SHOWN[s.kind]}
          </span>
        ),
      )}
    </span>
  );
});

/** The longest line of a message: a bar against the limit and "34 von 38"; over the limit in the warning colour with ⚠ and bold. */
function LongestLine({ width }: { width: number }) {
  const over = width > LINE_LIMIT;
  const fill = { "--text-line-fill": `${Math.min(100, (width / LINE_LIMIT) * 100)}%` } as CSSProperties;
  return (
    <span className={over ? `${styles.longest} ${styles.over}` : styles.longest}>
      <span className={styles.bar} style={fill} /* dyn */ aria-hidden="true" />
      {over ? (
        <span className={styles.count} data-over="">
          <Icon name="warning" weight="fill" />
          {`${width} von ${LINE_LIMIT}`}
        </span>
      ) : (
        <span className={styles.count}>{`${width} von ${LINE_LIMIT}`}</span>
      )}
    </span>
  );
}

/** The hand of the texts: the search, the messages (or the hits) as a table and the field of the selected message. */
export function TextHand() {
  const t = useTextArea();
  const { project, archive, message, loaded, error, loading, widths, query, effectiveScope, hits, searchError, select, setQuery, setScope, append } = t;
  const field = useRef<HTMLTextAreaElement>(null);
  const focusEditor = useCallback(() => field.current?.focus(), []);
  const messages = loaded?.messages;
  const q = query.trim();

  // In the scope of the open archive the search filters its messages here; the indices of the messages that match.
  const matches = useMemo(() => {
    if (!messages) return [];
    const all = messages.map((_, i) => i);
    if (q === "" || effectiveScope !== "archive") return all;
    const lower = q.toLowerCase();
    return all.filter((i) => messages[i]!.text.toLowerCase().includes(lower) || id3(i).includes(q));
  }, [messages, q, effectiveScope]);

  const selectMessage = useCallback((row: number) => archive !== null && select(archive, matches[row]!), [archive, matches, select]);
  const messageCells = useCallback(
    (row: number): ReactNode[] => {
      const i = matches[row]!;
      return [<span className={styles.number}>{id3(i)}</span>, <MessageText text={messages![i]!.text} />, <LongestLine width={widths[i] ?? 0} />];
    },
    [matches, messages, widths],
  );
  const messageAttrs = useCallback(
    (row: number) => {
      const i = matches[row]!;
      return q !== "" && archive !== null ? { "data-message": i, "data-hit": `${archive}:${i}` } : { "data-message": i };
    },
    [matches, q, archive],
  );

  const hitList: TextHit[] = useMemo(() => hits ?? [], [hits]);
  const selectHit = useCallback((row: number) => select(hitList[row]!.archive, hitList[row]!.index), [hitList, select]);
  const hitCells = useCallback(
    (row: number): ReactNode[] => {
      const h = hitList[row]!;
      return [<span className={styles.number}>{id4(h.archive)}</span>, <span className={styles.number}>{id3(h.index)}</span>, <span className={styles.text}>{h.excerpt}</span>];
    },
    [hitList],
  );
  const hitAttrs = useCallback((row: number) => ({ "data-hit": `${hitList[row]!.archive}:${hitList[row]!.index}` }), [hitList]);
  const selectedHit = hitList.findIndex((h) => h.archive === archive && h.index === message);

  if (project === null) {
    return (
      <div className={styles.empty}>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann stehen hier die Nachrichten seiner Textarchive." />
      </div>
    );
  }

  const nothing = (
    <div className={styles.empty}>
      <EmptyState what={`Nichts gefunden für „${q}“`} why="Prüfe die Schreibweise oder suche in allen Archiven." />
    </div>
  );
  let table: ReactNode;
  if (q !== "" && effectiveScope === "all") {
    if (searchError !== null) table = <Notice kind="error">{searchError}</Notice>;
    else if (hits === null) table = <p className={styles.note}>Suche …</p>;
    else if (hits.length === 0) table = nothing;
    else
      table = (
        <TextTable label="Treffer in allen Archiven" kind="hits" columns={HIT_COLUMNS} count={hits.length} selected={selectedHit < 0 ? null : selectedHit} rowAttrs={hitAttrs} cells={hitCells} onSelect={selectHit} onEnter={focusEditor} />
      );
  } else if (archive === null) {
    table = (
      <div className={styles.empty}>
        <EmptyState what="Wähle links ein Archiv." why="Dann stehen hier seine Nachrichten; die Suche oben sucht in allen Archiven." />
      </div>
    );
  } else if (error !== null) {
    table = <Notice kind="error">{error}</Notice>;
  } else if (loading || !messages) {
    table = <p className={styles.note}>Lade …</p>;
  } else if (messages.length === 0) {
    table = (
      <div className={styles.empty}>
        <EmptyState what="Dieses Archiv hat keine Nachricht." why="Lege eine an, dann kannst du sie hier schreiben." action={{ label: "Nachricht anlegen", onClick: append }} />
      </div>
    );
  } else if (matches.length === 0) {
    table = nothing;
  } else {
    const selectedRow = message === null ? -1 : matches.indexOf(message);
    table = (
      <TextTable
        label={`Nachrichten von Archiv ${id4(archive)}`}
        kind="messages"
        columns={MESSAGE_COLUMNS}
        count={matches.length}
        selected={selectedRow < 0 ? null : selectedRow}
        rowAttrs={messageAttrs}
        cells={messageCells}
        onSelect={selectMessage}
        onEnter={focusEditor}
      />
    );
  }

  return (
    <div className={styles.hand}>
      <div className={styles.head}>
        <div className={styles.search}>
          <TextField
            type="search"
            label="Texte suchen"
            placeholder={effectiveScope === "archive" ? "In diesem Archiv suchen" : "In allen Archiven suchen"}
            value={query}
            onChange={setQuery}
            onCommit={setQuery}
          />
        </div>
        <Segmented<TextScope> size="s" ariaLabel="Suchbereich" value={t.scope} onChange={setScope} options={SCOPES} />
        <Button kind="primary" icon="plus" disabled={!loaded} onClick={append}>
          Nachricht anlegen
        </Button>
      </div>
      <div className={styles.body}>
        <div className={styles.table}>{table}</div>
        {archive !== null && loaded && message !== null && loaded.messages[message] !== undefined && <EditorColumn archive={archive} message={message} field={field} />}
      </div>
    </div>
  );
}

/** The column of the selected message: its field, the headers that use the archive and the removal of the last message. */
function EditorColumn({ archive, message, field }: { archive: number; message: number; field: RefObject<HTMLTextAreaElement | null> }) {
  const { project, loaded, users, headers, commit, draft, setEditorFocused, removeLast, goTo } = useTextArea();
  const usedBy = users.get(archive) ?? [];
  // Each header under its own location name.
  const nameOf = (id: number) => headers.find((h) => h.id === id)?.location_name.trim() || "Header";
  const count = loaded?.messages.length ?? 0;
  const key = draftKey(archive, message);
  return (
    <aside className={styles.editor} aria-label="Nachricht bearbeiten">
      <PanelHead title={messageLabel(message)} subtitle={`Archiv ${id4(archive)}`} />
      <div className={styles.editorBody}>
        <MessageEditor
          key={key}
          value={loaded!.messages[message]!.text}
          onCommit={(text) => commit(project, archive, message, text)}
          onDraft={(text) => draft.set(key, text)}
          onFocusChange={setEditorFocused}
          fieldRef={field}
        />
        <div className={styles.usedIn}>
          <span className={styles.label}>Verwendet in</span>
          {usedBy.length === 0 ? (
            <span className={styles.none}>keinem Header</span>
          ) : (
            usedBy.map((id) => <Reference key={id} area="welt" label={nameOf(id)} tech={id4(id)} onOpen={() => goTo({ kind: "header", id })} dataAttrs={{ "data-header": id }} />)
          )}
        </div>
        <div className={styles.actions}>
          <Button kind="quiet" size="s" disabled={count <= 1} onClick={removeLast}>
            Letzte Nachricht entfernen
          </Button>
        </div>
      </div>
    </aside>
  );
}
