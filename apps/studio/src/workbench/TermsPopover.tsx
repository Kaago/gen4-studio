import { useCallback, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import { TERMS } from "./terms";
import { popoverStyle, useDismiss } from "./useDismiss";

/** Width of the glossary box; a narrower window gets the window's width less the margins. */
const WIDTH = 280;
/** Space kept to the window's edges. */
const MARGIN = 8;
/** The explorer's text: the box lives in the page body, outside the panel that sets it. */
const TEXT: CSSProperties = { fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" };

/**
 * Where the glossary box goes: under the button, its right edge at the button's, moved left or right as far as needed to
 * stay inside the window. It is placed in the window (fixed, rendered into the page body), not in the panel: a narrow
 * panel would cut it off, and the dock's panels are their own containing blocks.
 */
export function popoverPlace(button: { left: number; right: number; bottom: number }, windowWidth: number): CSSProperties {
  const width = Math.min(WIDTH, windowWidth - 2 * MARGIN);
  const left = Math.max(MARGIN, Math.min(button.right - width, windowWidth - MARGIN - width));
  return { position: "fixed", left, top: button.bottom + 4, width, boxSizing: "border-box" };
}

/** The glossary: a button that opens a list of every term of the world explorer with its technical name and explanation. */
export function TermsPopover() {
  const [place, setPlace] = useState<CSSProperties | null>(null);
  const open = place !== null;
  const wrapper = useRef<HTMLSpanElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const close = useCallback(() => setPlace(null), []);
  const inside = useMemo(() => [wrapper, box], []);
  useDismiss(open, inside, close);
  const toggle = () => {
    const rect = button.current?.getBoundingClientRect();
    setPlace(open || !rect ? null : popoverPlace(rect, window.innerWidth));
  };
  return (
    <span ref={wrapper} style={{ position: "relative" }}>
      <button ref={button} type="button" aria-haspopup="dialog" aria-expanded={open} onClick={toggle} title={`Was bedeuten ${TERMS.place.label}, ${TERMS.header.label}, ${TERMS.matrix.label} …?`}>
        Begriffe
      </button>
      {place !== null &&
        createPortal(
          <div ref={box} role="dialog" aria-label="Begriffe" style={{ ...popoverStyle, ...place, marginTop: 0, maxHeight: "60vh", overflowY: "auto", padding: "8px 12px", ...TEXT }}>
            <dl style={{ margin: 0 }}>
              {Object.entries(TERMS).map(([key, term]) => (
                <div key={key} style={{ margin: "0 0 8px" }}>
                  <dt style={{ fontWeight: 700 }}>
                    {term.label}
                    {term.tech && <span style={{ fontWeight: 400, color: "#6b7280" }}> ({term.tech})</span>}
                  </dt>
                  <dd style={{ margin: 0 }}>{term.short}</dd>
                </div>
              ))}
            </dl>
          </div>,
          document.body,
        )}
    </span>
  );
}
