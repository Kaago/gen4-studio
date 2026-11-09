import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
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
  /**
   * Only decoration: whoever shows the screen names what it shows beside it (a project card's button). The screen is then hidden from
   * assistive technology instead of being a group „DS-Bildschirm, …“.
   */
  decorative?: boolean;
  /** The group's name instead of „DS-Bildschirm, …“, for a screen that draws something of its own (a Pokédex profile, a battle preview). */
  label?: string;
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

export function DSScreen({ variant, pages, page, scale: given, background, decorative = false, label: named }: DSScreenProps) {
  const { t } = useTranslation("ui");
  const { frame, scale } = useScale(given);
  // No pages count as one empty page; a page index outside the list is clamped.
  const total = Math.max(1, pages.length);
  const index = Math.min(Math.max(0, Math.trunc(page)), total - 1);
  const shown = pages[index];
  const label = named ?? (variant === "empty" ? t("dsScreen.labelEmpty") : t("dsScreen.labelPage", { page: index + 1, total }));
  return (
    <div ref={frame} className={styles.frame} aria-hidden={decorative || undefined}>
      <div role={decorative ? undefined : "group"} aria-label={decorative ? undefined : label} className={styles.screen} style={{ "--ds-scale": scale } /* dyn */}>
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
              <span className={styles.overflow} role="img" aria-label={t("dsScreen.overflow")}>
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
