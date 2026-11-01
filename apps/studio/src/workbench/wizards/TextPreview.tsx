import { useState } from "react";
import { Button, DSScreen } from "../../ui/primitives";
import { pages } from "../text/textModel";
import styles from "./TextPreview.module.css";

/**
 * A text as the game shows it: the DS box with its pages (`pages`), „Vorige Seite“, „Seite n von m“ and „Nächste Seite“. The page stays
 * where it was while the text changes, unless the text got shorter.
 */
export function TextPreview({ text, variant = "textbox", label }: { text: string; variant?: "textbox" | "sign"; label?: string }) {
  const [page, setPage] = useState(0);
  const shown = pages(text);
  const total = shown.length;
  const index = Math.min(page, total - 1);
  return (
    <div className={styles.preview} data-figure-preview="">
      {label !== undefined && <span className={styles.caption}>{label}</span>}
      <DSScreen variant={variant} pages={shown} page={index} scale={1} />
      <div className={styles.pager}>
        <Button kind="quiet" size="s" aria-label="Vorige Seite" disabled={index === 0} onClick={() => setPage(index - 1)}>
          ‹
        </Button>
        <span className={styles.page}>{`Seite ${index + 1} von ${total}`}</span>
        <Button kind="quiet" size="s" aria-label="Nächste Seite" disabled={index >= total - 1} onClick={() => setPage(index + 1)}>
          ›
        </Button>
      </div>
    </div>
  );
}
