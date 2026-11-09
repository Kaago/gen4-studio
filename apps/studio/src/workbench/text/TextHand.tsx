import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode, RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { FigureDialog, HeaderDialogs, TextHit } from "../../api";
import { useProjectRefs } from "../../data/useRefs";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { Badge, Button, EmptyState, Icon, Notice, PanelHead, Reference, Segmented, TextField } from "../../ui/primitives";
import { depthAtLeast } from "../../ui/frame/shellState";
import { id4, termFor } from "../terms";
import { distinctMessages, figureSummary, figureTarget } from "../dialog/dialogModel";
import { DialogMessages, messageKey, useDialogWrites } from "../dialog/DialogMessages";
import { useDialog } from "../dialog/useDialog";
import { MessageEditor } from "./MessageEditor";
import { draftKey, useTextArea } from "./TextAreaContext";
import type { TextScope } from "./TextAreaContext";
import { TextTable } from "./TextTable";
import { LINE_LIMIT, id3, messageLabel, segments } from "./textModel";
import styles from "./TextHand.module.css";

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
  const { t } = useTranslation("workbench");
  const over = width > LINE_LIMIT;
  return (
    <span className={over ? `${styles.longest} ${styles.over}` : styles.longest}>
      <span className={styles.bar} style={{ "--text-line-fill": `${Math.min(100, (width / LINE_LIMIT) * 100)}%` }} /* dyn */ aria-hidden="true" />
      {over ? (
        <span className={styles.count} data-over="">
          <Icon name="warning" weight="fill" />
          {t("textHand.lineWidth", { width, limit: LINE_LIMIT })}
        </span>
      ) : (
        <span className={styles.count}>{t("textHand.lineWidth", { width, limit: LINE_LIMIT })}</span>
      )}
    </span>
  );
}

/** The hand of the texts: in Erzählen the messages of the chosen figure (`DialogHand`), from Bauen on the archive's messages. */
export function TextHand() {
  return useTextArea().narrating ? <DialogHand /> : <ArchiveHand />;
}

/** The search, the messages (or the hits) as a table and the field of the selected message. */
function ArchiveHand() {
  const { t } = useTranslation("workbench");
  const area = useTextArea();
  const reset = useReset();
  const { project, archive, message, loaded, error, loading, widths, query, effectiveScope, hits, searchError, select, setQuery, setScope, append } = area;
  const messageColumns = useMemo(() => [{ title: t("textHand.columns.number") }, { title: t("textHand.columns.text") }, { title: t("textHand.columns.longestLine") }], [t]);
  const hitColumns = useMemo(() => [{ title: t("textHand.columns.archive") }, { title: t("textHand.columns.number") }, { title: t("textHand.columns.text") }], [t]);
  const scopes = useMemo(
    () => [
      { value: "archive" as const, label: t("textHand.scope.archive") },
      { value: "all" as const, label: t("textHand.scope.all") },
    ],
    [t],
  );
  const field = useRef<HTMLTextAreaElement>(null);
  // The message whose field takes the focus as soon as it shows (Enter on a row: the table selected it, its archive may still load).
  const focusWanted = useRef<string | null>(null);
  const shownKey = archive !== null && message !== null && loaded?.messages[message] !== undefined ? draftKey(archive, message) : null;
  const focusWhenShown = useCallback(
    (key: string) => {
      focusWanted.current = key;
      // The row was selected already: its field is there, nothing renders again.
      if (key === shownKey && field.current) {
        focusWanted.current = null;
        field.current.focus();
      }
    },
    [shownKey],
  );
  useEffect(() => {
    if (focusWanted.current === null || focusWanted.current !== shownKey || !field.current) return;
    focusWanted.current = null;
    field.current.focus();
  });
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
  // Enter on a row: the table selected its message; its field takes the focus once it shows.
  const enterMessage = useCallback((row: number) => archive !== null && focusWhenShown(draftKey(archive, matches[row]!)), [archive, matches, focusWhenShown]);
  const enterHit = useCallback((row: number) => focusWhenShown(draftKey(hitList[row]!.archive, hitList[row]!.index)), [hitList, focusWhenShown]);

  if (project === null) {
    return (
      <div className={styles.empty}>
        <EmptyState what={t("textHand.noProject.what")} why={t("textHand.noProject.why")} />
      </div>
    );
  }

  const nothing = (
    <div className={styles.empty}>
      <EmptyState what={t("textHand.nothingFound.what", { query: q })} why={t("textHand.nothingFound.why")} />
    </div>
  );
  let table: ReactNode;
  if (q !== "" && effectiveScope === "all") {
    if (searchError !== null) table = <Notice kind="error">{searchError}</Notice>;
    else if (hits === null) table = <p className={styles.note}>{t("textHand.searching")}</p>;
    else if (hits.length === 0) table = nothing;
    else
      table = (
        <TextTable key="hits" label={t("textHand.hitsLabel")} kind="hits" columns={hitColumns} count={hits.length} selected={selectedHit < 0 ? null : selectedHit} rowAttrs={hitAttrs} cells={hitCells} onSelect={selectHit} onEnter={enterHit} />
      );
  } else if (archive === null) {
    table = (
      <div className={styles.empty}>
        <EmptyState what={t("textHand.noArchive.what")} why={t("textHand.noArchive.why")} />
      </div>
    );
  } else if (error !== null) {
    table = <Notice kind="error">{error}</Notice>;
  } else if (loading || !messages) {
    table = <p className={styles.note}>{t("textHand.loading")}</p>;
  } else if (messages.length === 0) {
    table = (
      <div className={styles.empty}>
        <EmptyState what={t("textHand.emptyArchive.what")} why={t("textHand.emptyArchive.why")} action={{ label: t("textHand.addMessage"), onClick: append }} />
      </div>
    );
  } else if (matches.length === 0) {
    table = nothing;
  } else {
    const selectedRow = message === null ? -1 : matches.indexOf(message);
    // Keyed by the archive: another archive's table starts at its top (scroll position and row window are its own).
    table = (
      <TextTable
        key={`archive-${archive}`}
        label={t("textHand.messagesLabel", { archive: id4(archive) })}
        kind="messages"
        columns={messageColumns}
        count={matches.length}
        selected={selectedRow < 0 ? null : selectedRow}
        rowAttrs={messageAttrs}
        cells={messageCells}
        onSelect={selectMessage}
        onEnter={enterMessage}
      />
    );
  }

  return (
    <div className={styles.hand}>
      <div className={styles.head}>
        <div className={styles.search}>
          <TextField
            type="search"
            label={t("textHand.search.label")}
            placeholder={effectiveScope === "archive" ? t("textHand.search.inArchive") : t("textHand.search.inAll")}
            value={query}
            onChange={setQuery}
            onCommit={setQuery}
          />
        </div>
        <Segmented<TextScope> size="s" ariaLabel={t("textHand.search.scope")} value={area.scope} onChange={setScope} options={scopes} />
        <Button kind="primary" icon="plus" disabled={!loaded} onClick={append}>
          {t("textHand.addMessage")}
        </Button>
        {archive !== null && (
          <HeadActions
            push={false}
            items={[
              // The selected message's text (a message the base does not have is refused by the plan, which the dialog shows).
              message === null
                ? { label: t("textHand.reset.message"), icon: "undo", disabled: true, detail: t("textHand.reset.noMessage"), onSelect: () => {} }
                : resetEntry({ kind: "text_message", archive, message }, reset.status, reset.open, t("textHand.reset.message")),
              resetEntry({ kind: "text_archive", index: archive }, reset.status, reset.open, t("textHand.reset.archive")),
            ]}
          />
        )}
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
  const { t } = useTranslation("workbench");
  const { project, loaded, users, headers, commit, draft, setEditorFocused, removeLast, goTo, depth } = useTextArea();
  const usedBy = users.get(archive) ?? [];
  // Each header under its own location name ("Ort" in Bauen, "Header" in Tüfteln when it has none).
  const word = termFor("header", depth);
  const nameOf = (id: number) => headers.find((h) => h.id === id)?.location_name.trim() || word;
  const count = loaded?.messages.length ?? 0;
  const key = draftKey(archive, message);
  return (
    <aside className={styles.editor} aria-label={t("textHand.editor.label")}>
      <PanelHead title={messageLabel(message)} subtitle={t("textHand.editor.archive", { archive: id4(archive) })} />
      <div className={styles.editorBody}>
        <MessageEditor
          key={key}
          value={loaded!.messages[message]!.text}
          onCommit={(text) => commit(project, archive, message, text)}
          onDraft={(text) => draft.set(key, text)}
          onFocusChange={setEditorFocused}
          fieldRef={field}
        />
        <MessageRefs archive={archive} message={message} />
        <div className={styles.usedIn}>
          <span className={styles.label}>{t("textHand.editor.usedIn")}</span>
          {usedBy.length === 0 ? (
            <span className={styles.none}>{depthAtLeast(depth, "tuefteln") ? t("textHand.editor.noHeader") : t("textHand.editor.noPlace")}</span>
          ) : (
            usedBy.map((id) => <Reference key={id} area="welt" label={nameOf(id)} tech={id4(id)} onOpen={() => goTo({ kind: "header", id })} dataAttrs={{ "data-header": id }} />)
          )}
        </div>
        <div className={styles.actions}>
          <Button kind="quiet" size="s" disabled={count <= 1} onClick={removeLast}>
            {t("textHand.editor.removeLast")}
          </Button>
        </div>
      </div>
    </aside>
  );
}

/**
 * Where the selected message is used („Nachricht verwendet in“, apart from the archive's „Verwendet in“ below): a chip per line of a script source that shows it (`refs` of the message, 3d.5), which opens the
 * script panel at that line. Read when another message is chosen.
 */
function MessageRefs({ archive, message }: { archive: number; message: number }) {
  const { t } = useTranslation("workbench");
  const { project, goTo } = useTextArea();
  const target = useMemo(() => ({ kind: "message" as const, archive, index: message }), [archive, message]);
  const found = useProjectRefs(project, target);
  let chips: ReactNode;
  if (found === null) chips = <span className={styles.none}>…</span>;
  else if (found.error !== null) chips = <span className={styles.none}>{found.error}</span>;
  else if (found.refs!.length === 0) chips = <span className={styles.none}>{t("textHand.refs.none")}</span>;
  else
    chips = found.refs!.map((r) => {
      const line = r.nav.kind === "script" ? r.nav.line : undefined;
      return (
        <Reference
          key={`${r.source.index}:${r.slot ?? ""}:${r.field}`}
          area="welt"
          label={r.source_name}
          tech={line === undefined ? undefined : t("textHand.refs.line", { line })}
          onOpen={() => goTo(r.nav)}
          dataAttrs={{ "data-script-line": `${r.source.index}:${line ?? ""}` }}
        />
      );
    });
  return (
    <div className={styles.usedIn} data-message-refs="">
      <span className={styles.label}>{t("textHand.refs.label")}</span>
      {chips}
    </div>
  );
}

/**
 * The hand of the texts in Erzählen (`[data-dialog-hand]`): the messages of the chosen figure as the dialog editor shows them (`DialogMessages`:
 * a field per message, „Geteilt mit N Figuren“, „Für diese Figur abspalten“) and the way into the editor. A trainer's texts are the trainer's
 * (Pokémon); a common script, a figure without a script and one that cannot be read say so.
 */
function DialogHand() {
  const { t } = useTranslation("workbench");
  const { project, dialogs, dialogFigure, figureTitle, openDialogEditor, goTo } = useTextArea();
  let body: ReactNode;
  let actions: ReactNode = undefined;
  if (project === null) {
    body = <EmptyState what={t("textHand.noProject.what")} why={t("textHand.dialog.noProjectWhy")} />;
  } else if (dialogFigure === null || dialogs.dialogs === null) {
    body = <EmptyState what={t("textHand.dialog.noFigure.what")} why={t("textHand.dialog.noFigure.why")} />;
  } else if (dialogFigure.source.kind === "local") {
    const figure = dialogFigure;
    actions = (
      <Button kind="quiet" size="s" onClick={() => openDialogEditor(figure)}>
        {t("textHand.dialog.edit")}
      </Button>
    );
    body = <FigureMessages key={`${dialogs.dialogs.header}:${figure.kind}:${figure.index}`} dialogs={dialogs.dialogs} figure={figure} />;
  } else {
    const source = dialogFigure.source;
    if (source.kind === "trainer")
      actions = (
        <Button kind="quiet" size="s" icon="forward" onClick={() => goTo({ kind: "trainer", index: source.id })}>
          {t("textHand.dialog.openTrainer")}
        </Button>
      );
    body = <p className={styles.note}>{figureSummary(dialogFigure)}</p>;
  }
  return (
    <div className={styles.hand} data-dialog-hand="">
      {dialogFigure !== null && <PanelHead title={figureTitle(dialogFigure)} actions={actions} />}
      <div className={styles.dialogBody}>{body}</div>
    </div>
  );
}

/** The messages of a figure with a script of its own, read with `dialogOf` and written like in the dialog editor. */
function FigureMessages({ dialogs, figure }: { dialogs: HeaderDialogs; figure: FigureDialog }) {
  const { t } = useTranslation("workbench");
  const { dialogRevision, dialogChanged, onError, scriptFiles } = useTextArea();
  const target = useMemo(() => figureTarget(dialogs, figure), [dialogs, figure]);
  const { dialog, error, reload } = useDialog(target, dialogRevision);
  const [chosen, setChosen] = useState<string | null>(null);
  const source = dialog?.source ?? null;
  const writes = useDialogWrites(target, source, { reload, onChanged: dialogChanged, onError, scriptFiles, onSplit: setChosen, narrating: true });
  const distinct = useMemo(() => (dialog !== null && source?.kind === "local" ? distinctMessages(dialog.messages) : []), [dialog, source?.kind]);
  if (error !== null) return <Notice kind="error">{error}</Notice>;
  if (dialog === null) return <p className={styles.note}>{t("textHand.loading")}</p>;
  if (source?.kind !== "local") return <p className={styles.note}>{figureSummary(figure)}</p>;
  if (distinct.length === 0) return <p className={styles.note}>{t("textHand.dialog.noMessage")}</p>;
  const currentKey = distinct.some((d) => messageKey(d.message) === chosen) ? chosen : messageKey(distinct[0]!.message);
  return (
    <>
      {writes.notice !== null && <Notice kind={writes.notice.kind}>{writes.notice.text}</Notice>}
      <DialogMessages kind={figure.kind} distinct={distinct} currentKey={currentKey} onChoose={setChosen} writes={writes} narrating />
    </>
  );
}
