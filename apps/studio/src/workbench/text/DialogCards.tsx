import { memo, useCallback, useEffect, useId, useMemo } from "react";
import type { FigureDialog } from "../../api";
import { Button, DSScreen, EmptyState, Notice, PanelHead } from "../../ui/primitives";
import { figureKey, figureSummary, speakersText } from "../dialog/dialogModel";
import { headerIdentity } from "../headerIdentity";
import { useTextArea } from "./TextAreaContext";
import { pages } from "./textModel";
import styles from "./DialogCards.module.css";

/** One figure: the first page of what it says on the DS screen, its title and line, and the way to its dialog (or its trainer). */
const FigureCard = memo(function FigureCard({
  figure,
  title,
  selected,
  onSelect,
  onEdit,
  onTrainer,
}: {
  figure: FigureDialog;
  title: string;
  selected: boolean;
  onSelect: (figure: FigureDialog) => void;
  onEdit: (figure: FigureDialog) => void;
  onTrainer: (id: number) => void;
}) {
  const lineId = useId();
  const first = useMemo(() => (figure.first_text === null ? [] : pages(figure.first_text).slice(0, 1)), [figure.first_text]);
  const source = figure.source;
  const silent = source.kind === "none" || (source.kind === "local" && figure.messages === 0);
  const variant = first.length === 0 ? "empty" : figure.kind === "bg" ? "sign" : "textbox";
  // A double click does what the card's key does: the dialog editor of a script of the figure's own, the trainer of a trainer.
  const open = source.kind === "local" ? () => onEdit(figure) : source.kind === "trainer" ? () => onTrainer(source.id) : undefined;
  return (
    <li
      className={styles.card}
      data-dialog-figure={figureKey(figure)}
      data-selected={selected ? "" : undefined}
      data-muted={silent ? "" : undefined}
      onClick={() => onSelect(figure)}
      onDoubleClick={open}
    >
      <div className={styles.screen}>
        <DSScreen variant={variant} pages={first} page={0} scale={1} />
      </div>
      <button type="button" className={styles.pick} aria-pressed={selected} aria-label={title} aria-describedby={lineId}>
        <span className={styles.title}>{title}</span>
        <span id={lineId} className={styles.line}>
          {figureSummary(figure)}
        </span>
      </button>
      {source.kind === "local" && (
        <div className={styles.actions}>
          <Button kind="quiet" size="s" onClick={() => onEdit(figure)}>
            Dialog bearbeiten…
          </Button>
        </div>
      )}
      {source.kind === "trainer" && (
        <div className={styles.actions}>
          <Button kind="quiet" size="s" icon="forward" onClick={() => onTrainer(source.id)}>
            Trainer öffnen
          </Button>
        </div>
      )}
    </li>
  );
});

/**
 * The view of the texts in Erzählen (`[data-dialog-cards]`): one card per figure of the chosen place. A click chooses the figure (the hand
 * shows its messages); a double click or „Dialog bearbeiten…“ opens the dialog editor, a trainer's card leads to the trainer instead.
 */
export function DialogCards() {
  const { project, places, dialogHeader, dialogs, dialogFigure, headers, figureTitle, selectFigure, openDialogEditor, goTo, needTrainers } = useTextArea();
  const figures = dialogs.dialogs?.figures;
  const hasTrainer = figures?.some((f) => f.source.kind === "trainer") ?? false;
  useEffect(() => {
    if (hasTrainer) needTrainers();
  }, [hasTrainer, needTrainers]);
  const header = dialogHeader === null ? undefined : headers.find((h) => h.id === dialogHeader);
  const onTrainer = useCallback((id: number) => goTo({ kind: "trainer", index: id }), [goTo]);

  let empty: { what: string; why: string } | null = null;
  if (project === null) empty = { what: "Kein Projekt geöffnet.", why: "Öffne ein Projekt, dann zeigt die Schau die Figuren seiner Orte." };
  else if (dialogHeader === null || header === undefined) empty = { what: "Wähle links einen Ort.", why: "Die Schau zeigt dann seine Figuren mit dem, was sie sagen." };
  if (empty !== null || header === undefined) {
    return (
      <div className={styles.empty}>
        <EmptyState what={empty?.what ?? ""} why={empty?.why ?? ""} />
      </div>
    );
  }

  let body;
  if (figures === undefined) {
    body = dialogs.error !== null ? <Notice kind="error">{dialogs.error}</Notice> : <p className={styles.note}>Lade …</p>;
  } else if (figures.length === 0) {
    body = (
      <div className={styles.empty}>
        <EmptyState what="Keine Figuren an diesem Ort" why="Hier steht keine Figur und kein Schild, das etwas sagt." />
      </div>
    );
  } else {
    body = (
      <ul className={styles.grid} aria-label="Figuren">
        {figures.map((f) => (
          <FigureCard
            key={figureKey(f)}
            figure={f}
            title={figureTitle(f)}
            selected={dialogFigure !== null && figureKey(dialogFigure) === figureKey(f)}
            onSelect={selectFigure}
            onEdit={openDialogEditor}
            onTrainer={onTrainer}
          />
        ))}
      </ul>
    );
  }
  return (
    <section className={styles.view} aria-label="Figuren des Orts" data-dialog-cards="">
      <PanelHead title={headerIdentity(places, header).name} subtitle={figures === undefined ? undefined : speakersText(figures)} />
      <div className={styles.body}>{body}</div>
    </section>
  );
}
