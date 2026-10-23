import { useEffect, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import type { Page, Segment } from "../../workbench/text/textModel";
import { Badge } from "./Badge";
import { Icon } from "./Icon";
import styles from "./DSScreen.module.css";

/** The DS screen is 256 × 192 pixels; `scale` multiplies them. */
const WIDTH = 256;

const UNKNOWN_CODE = /^\\x[0-9a-fA-F]{4}$/;

export type DSScreenProps = {
  /** `textbox`: dialog box; `sign`: the same box in the sign colours; `empty`: only the ground. */
  variant: "textbox" | "sign" | "empty";
  pages: Page[];
  /** Index of the shown page. */
  page: number;
  /** Whole-number magnification; measured from the container width (at least 1) when missing. */
  scale?: number;
  /** Map picture or similar, drawn on the ground behind the box. */
  background?: ReactNode;
};

function useScale(given: number | undefined) {
  const frame = useRef<HTMLDivElement>(null);
  const [measured, setMeasured] = useState(1);
  useEffect(() => {
    const el = frame.current;
    if (given !== undefined || !el || typeof ResizeObserver === "undefined") return;
    const measure = () => setMeasured(Math.max(1, Math.floor(el.clientWidth / WIDTH)));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [given]);
  return { frame, scale: given === undefined ? measured : Math.max(1, Math.round(given)) };
}

/** Plain text shows an escaped backslash (`\\`) as one; a code becomes a badge, an unknown `\xNNNN` a "?" that names the code. */
function Part({ segment }: { segment: Segment }) {
  if (segment.kind === "code") {
    return UNKNOWN_CODE.test(segment.text) ? (
      <Badge kind="code" title={segment.text}>
        ?
      </Badge>
    ) : (
      <Badge kind="code">{segment.text}</Badge>
    );
  }
  return <>{segment.text.replace(/\\\\/g, "\\")}</>;
}

export function DSScreen({ variant, pages, page, scale: given, background }: DSScreenProps) {
  const { frame, scale } = useScale(given);
  // No pages count as one empty page; a page index outside the list is clamped.
  const total = Math.max(1, pages.length);
  const index = Math.min(Math.max(0, Math.trunc(page)), total - 1);
  const shown = pages[index];
  const label = variant === "empty" ? "DS-Bildschirm, leer" : `DS-Bildschirm, Seite ${index + 1} von ${total}`;
  return (
    <div ref={frame} className={styles.frame}>
      <div role="group" aria-label={label} className={styles.screen} style={{ "--ds-scale": scale } as CSSProperties /* dyn */}>
        {background !== undefined && <div className={styles.background}>{background}</div>}
        {variant !== "empty" && shown && (
          <div className={`${styles.box} ${variant === "sign" ? styles.sign : styles.textbox}`}>
            {shown.lines.map((line, i) => (
              <div key={i} className={styles.line}>
                {line.map((segment, j) => (
                  <Part key={j} segment={segment} />
                ))}
              </div>
            ))}
            {shown.overflow && (
              <span className={styles.overflow} role="img" aria-label="Zeile läuft aus der Box">
                <Icon name="warning" weight="fill" />
              </span>
            )}
            {shown.prompt && (
              <span className={styles.prompt} aria-hidden="true">
                ▼
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
