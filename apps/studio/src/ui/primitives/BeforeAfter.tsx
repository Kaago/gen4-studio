import { createElement, useLayoutEffect, useMemo, useRef } from "react";
import styles from "./BeforeAfter.module.css";

export type BeforeAfterProps = {
  before: string;
  after: string;
  /** Names the comparison (what changes, "Textarchiv 0433"); default „Vorher und Nachher“. */
  label?: string;
  /**
   * Long lines break inside their column instead of scrolling sideways; a wrapped line's continuation rows hang in, so it still reads
   * as one line. For lines that are long but carry no layout of their own (a message, an event as JSON, a map's model line).
   */
  wrap?: boolean;
};

/** The lines of a text; none for an empty one. */
const linesOf = (text: string) => (text === "" ? [] : text.split("\n"));

/** Above this many line pairs every line counts as changed (the table of the comparison would grow too big). */
const MAX_PAIRS = 250_000;

/** Which lines of `a` and of `b` are outside their longest common sequence of lines: what went and what came. */
export function changedLines(a: readonly string[], b: readonly string[]): { before: boolean[]; after: boolean[] } {
  const before = a.map(() => true);
  const after = b.map(() => true);
  if (a.length * b.length > MAX_PAIRS) return { before, after };
  // common[i][j]: length of the longest common sequence of a[i..] and b[j..].
  const common = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) common[i]![j] = a[i] === b[j] ? common[i + 1]![j + 1]! + 1 : Math.max(common[i + 1]![j]!, common[i]![j + 1]!);
  }
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      before[i++] = false;
      after[j++] = false;
    } else if (common[i + 1]![j]! >= common[i]![j + 1]!) i++;
    else j++;
  }
  return { before, after };
}

/** Used when the computed line height is not a number ("normal"). */
const FALLBACK_LINE_HEIGHT = 16;

/**
 * Scrolls a column to its first changed line, which ends up two visual lines below the column's top (some context above it, whatever
 * the height of the line before: with `wrap` that can be a whole wrapped event). The column's own `scrollTop` is set (no `scrollIntoView`:
 * it would scroll the dialog and the page too). `offsetTop` measures from the column (`.lines` is positioned), so it is exact whatever
 * transform an opening dialog still has. A change that is in view already leaves the column on top.
 */
function showFirstChange(list: HTMLElement) {
  const first = list.querySelector<HTMLElement>("[data-changed]");
  if (!first || first.offsetTop + first.offsetHeight <= list.clientHeight) {
    list.scrollTop = 0;
    return;
  }
  const lineHeight = parseFloat(getComputedStyle(first).lineHeight) || FALLBACK_LINE_HEIGHT;
  list.scrollTop = Math.max(0, first.offsetTop - 2 * lineHeight);
}

function Side({ title, lines, changed, mark }: { title: string; lines: readonly string[]; changed: readonly boolean[]; mark: "del" | "ins" }) {
  const list = useRef<HTMLOListElement>(null);
  // On mount and when the text changes: the difference is the point, not the unchanged top of a long text.
  useLayoutEffect(() => {
    const box = list.current;
    if (!box) return;
    showFirstChange(box);
    // The column may still settle (a dialog opening, fonts, the final width: wrapped lines change height and the change moves): jump
    // again on every resize, until the user takes the scroll over. Their input, not the scroll event, stops it: our own jump fires one too.
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => showFirstChange(box));
    observer.observe(box);
    const first = box.querySelector("[data-changed]");
    if (first) observer.observe(first);
    const stop = () => observer.disconnect();
    const inputs = ["wheel", "touchstart", "pointerdown", "keydown"] as const;
    for (const type of inputs) box.addEventListener(type, stop, { once: true, passive: true });
    return () => {
      stop();
      for (const type of inputs) box.removeEventListener(type, stop);
    };
  }, [lines, changed]);
  return (
    <div role="group" aria-label={title} className={styles.side}>
      <div className={styles.head} aria-hidden="true">
        {title}
      </div>
      {lines.length === 0 ? (
        <p className={styles.empty}>Leer</p>
      ) : (
        <ol ref={list} className={styles.lines}>
          {lines.map((line, i) =>
            changed[i] ? (
              <li key={i} data-line="" data-changed="" className={`${styles.line} ${styles.changed} ${mark === "del" ? styles.removed : styles.added}`}>
                {createElement(mark, { className: styles.text }, line)}
              </li>
            ) : (
              <li key={i} data-line="" className={styles.line}>
                <span className={styles.text}>{line}</span>
              </li>
            ),
          )}
        </ol>
      )}
    </div>
  );
}

/**
 * A text before and after a change in two columns (the preview of a fix). Lines that went are a deletion in „Vorher“, lines that came an
 * insertion in „Nachher“: highlighted in the selection surface and marked with − and +, never by colour alone. With `wrap` long lines
 * break inside their column.
 */
export function BeforeAfter({ before, after, label, wrap = false }: BeforeAfterProps) {
  const a = useMemo(() => linesOf(before), [before]);
  const b = useMemo(() => linesOf(after), [after]);
  const changed = useMemo(() => changedLines(a, b), [a, b]);
  return (
    <div role="group" aria-label={label ?? "Vorher und Nachher"} className={wrap ? `${styles.frame} ${styles.wrap}` : styles.frame} data-wrap={wrap ? "" : undefined}>
      <Side title="Vorher" lines={a} changed={changed.before} mark="del" />
      <Side title="Nachher" lines={b} changed={changed.after} mark="ins" />
    </div>
  );
}
