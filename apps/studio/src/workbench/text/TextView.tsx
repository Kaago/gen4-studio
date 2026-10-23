import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { CSSProperties, KeyboardEvent, RefObject } from "react";
import type { HeaderSummary } from "../../api";
import { Badge, Button, DSScreen, EmptyState, PanelHead } from "../../ui/primitives";
import { thumbnails } from "../../world/thumbnailStore";
import { useThumbnail } from "../../world/useThumbnail";
import { mapOfHeader } from "../headers";
import { id4 } from "../terms";
import { useMatrix } from "../useMatrix";
import { draftKey, useTextArea } from "./TextAreaContext";
import { LINE_LIMIT, lineWidth, messageLabel, pages, visibleLine } from "./textModel";
import type { Segment } from "./textModel";
import styles from "./TextView.module.css";

/** The DS screen is 256 × 192 pixels. */
const DS_WIDTH = 256;
const DS_HEIGHT = 192;
/** The magnification while the space cannot be measured (jsdom). */
const SCALE_FALLBACK = 2;

const ignoreError = () => {};

/** The whole-number magnification of the DS screen that fits `ref`'s box. */
function useFitScale(ref: RefObject<HTMLElement | null>): number {
  const [scale, setScale] = useState(SCALE_FALLBACK);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      if (el.clientWidth === 0 || el.clientHeight === 0) return;
      setScale(Math.max(1, Math.floor(Math.min(el.clientWidth / DS_WIDTH, el.clientHeight / DS_HEIGHT))));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return scale;
}

/** The picture of the map of the first header that uses the archive, behind the text box (nothing while it is not there). */
function MapPicture({ header }: { header: HeaderSummary }) {
  const matrix = useMatrix(header.matrix_id, 0, ignoreError);
  const map = matrix ? mapOfHeader(matrix, header.id) : null;
  const picture = useThumbnail(thumbnails, map, header.area_data_id);
  if (typeof picture !== "string") return null;
  const style: CSSProperties = { backgroundImage: `url("${picture}")` };
  return <div className={styles.map} style={style} /* dyn */ data-map-picture="" />;
}

/** The characters of one line as a card writes them: the line and "34 von 38". */
function CardLine({ line }: { line: Segment[] }) {
  const width = lineWidth(line);
  const over = width > LINE_LIMIT;
  return (
    <li className={styles.cardLine}>
      <span className={styles.cardText}>{visibleLine(line.map((s) => s.text).join("")) || " "}</span>
      <span className={over ? `${styles.width} ${styles.over}` : styles.width} data-line-width="" data-over={over ? "" : undefined}>
        {`${width} von ${LINE_LIMIT}`}
      </span>
    </li>
  );
}

/** The selected message on the DS screen, page by page, with its pages as cards beside it. */
function MessagePreview({ archive, message, stored }: { archive: number; message: number; stored: string }) {
  const { page, setPage, draft, headers, users, rows } = useTextArea();
  // What is typed in the field shows before it is committed.
  useSyncExternalStore(draft.subscribe, draft.get);
  const text = draft.textOf(draftKey(archive, message)) ?? stored;
  const all = useMemo(() => pages(text), [text]);
  const shown = Math.min(Math.max(0, page), all.length - 1);
  const stage = useRef<HTMLDivElement>(null);
  const scale = useFitScale(stage);
  const firstUser = users.get(archive)?.[0];
  const header = firstUser === undefined ? undefined : headers.find((h) => h.id === firstUser);
  const place = rows[archive]?.place ?? null;

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
    if (event.key === "ArrowRight" && shown < all.length - 1) setPage(shown + 1);
    else if (event.key === "ArrowLeft" && shown > 0) setPage(shown - 1);
    else return;
    event.preventDefault();
  }

  return (
    <section className={styles.view} aria-label="Vorschau" tabIndex={-1} onKeyDown={onKeyDown}>
      <PanelHead
        title={messageLabel(message)}
        subtitle={place ? `Archiv ${id4(archive)}, ${place}` : `Archiv ${id4(archive)}`}
        actions={<Badge kind="neutral">{all.length === 1 ? "1 Seite" : `${all.length} Seiten`}</Badge>}
      />
      <div className={styles.body}>
        <div className={styles.screenColumn}>
          <div ref={stage} className={styles.stage}>
            <DSScreen variant="textbox" pages={all} page={shown} scale={scale} background={header ? <MapPicture header={header} /> : undefined} />
          </div>
          <div className={styles.controls}>
            <span className={styles.facts}>{`Seite ${shown + 1} von ${all.length}, ${DS_WIDTH} × ${DS_HEIGHT}, ${scale}-fach`}</span>
            <Button kind="quiet" icon="back" disabled={shown === 0} onClick={() => setPage(shown - 1)}>
              Zurück
            </Button>
            <Button kind="secondary" disabled={shown >= all.length - 1} onClick={() => setPage(shown + 1)}>
              Weiterblättern
            </Button>
          </div>
        </div>
        <ol className={styles.cards} aria-label="Seiten">
          {all.map((p, i) => (
            <li key={i}>
              <button type="button" className={i === shown ? `${styles.card} ${styles.current}` : styles.card} aria-pressed={i === shown} data-page={i} onClick={() => setPage(i)}>
                <span className={styles.cardTitle} data-page-title="">{`Seite ${i + 1}`}</span>
                <ul className={styles.cardLines}>
                  {p.lines.map((line, j) => (
                    <CardLine key={j} line={line} />
                  ))}
                </ul>
              </button>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}

/** The view of the texts (Schau): the selected message on the DS screen; what is missing otherwise. */
export function TextView() {
  const { project, archive, message, loaded, loading, error } = useTextArea();
  let empty: { what: string; why: string } | null = null;
  if (project === null) empty = { what: "Kein Projekt geöffnet.", why: "Öffne ein Projekt, dann zeigt die Vorschau seine Texte." };
  else if (archive === null) empty = { what: "Wähle links ein Archiv.", why: "Die Vorschau zeigt dann seine Nachrichten auf dem DS-Bildschirm." };
  else if (error !== null) empty = { what: "Dieses Archiv ist nicht lesbar.", why: "Die Nachrichten unten nennen den Grund." };
  else if (loading || !loaded) empty = { what: "Lade …", why: "Das Archiv wird gelesen." };
  else if (message === null) empty = { what: "Dieses Archiv hat keine Nachricht.", why: "Lege unten eine Nachricht an, dann zeigt die Vorschau sie." };
  if (empty !== null || archive === null || message === null || !loaded) {
    return (
      <div className={styles.empty}>
        <EmptyState what={empty?.what ?? ""} why={empty?.why ?? ""} />
      </div>
    );
  }
  return <MessagePreview archive={archive} message={message} stored={loaded.messages[message]!.text} />;
}
