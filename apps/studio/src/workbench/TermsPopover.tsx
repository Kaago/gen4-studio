import { useCallback, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { TERMS } from "./terms";
import { useDismiss } from "./useDismiss";

export const popoverStyle: CSSProperties = {
  position: "absolute",
  top: "100%",
  zIndex: 10,
  marginTop: 4,
  background: "#fff",
  border: "1px solid #d1d5db",
  borderRadius: 6,
  boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
};

/** The glossary: a button that opens a list of every term of the world explorer with its technical name and explanation. */
export function TermsPopover() {
  const [open, setOpen] = useState(false);
  const wrapper = useRef<HTMLSpanElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useDismiss(open, wrapper, close);
  return (
    <span ref={wrapper} style={{ position: "relative" }}>
      <button type="button" aria-haspopup="dialog" aria-expanded={open} onClick={() => setOpen((o) => !o)} title="Was bedeuten Ort, Header, Raster …?">
        Begriffe
      </button>
      {open && (
        <div role="dialog" aria-label="Begriffe" style={{ ...popoverStyle, right: 0, width: 280, maxHeight: "60vh", overflowY: "auto", padding: "8px 12px" }}>
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
        </div>
      )}
    </span>
  );
}
