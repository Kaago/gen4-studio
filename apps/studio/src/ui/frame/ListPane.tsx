import { useRef } from "react";
import { useTranslation } from "react-i18next";
import type { ReactNode } from "react";
import { PanelHead } from "../primitives";
import type { PanelHeadSwitcher } from "../primitives";
import styles from "./ListPane.module.css";

export type ListPaneProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  /** The parts of the area; the head's title becomes a menu that switches between them. */
  switcher?: PanelHeadSwitcher;
  /** Whether the list shows (272 px) or is folded to its 8 px grip. */
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
};

/**
 * The list column of an area: a `PanelHead`, the content below it and a grip at the right edge. A double click on the
 * grip folds the list (Enter or Space on it too); a click on the folded grip opens it. The content stays mounted while
 * folded, so a search, the opened places and the scroll position survive.
 */
export function ListPane({ title, subtitle, actions, switcher, open, onToggle, children }: ListPaneProps) {
  const { t } = useTranslation("ui");
  // Whether the list was open when the current press began: the double click that opened the list must not fold it again.
  const openAtPress = useRef(true);
  return (
    <div className={styles.pane} data-open={open}>
      <section className={styles.body} aria-label={title} hidden={!open}>
        <PanelHead title={title} subtitle={subtitle} actions={actions} switcher={switcher} />
        <div className={styles.content}>{children}</div>
      </section>
      <button
        type="button"
        className={styles.grip}
        aria-label={open ? t("listPane.collapse") : t("listPane.expand")}
        aria-expanded={open}
        title={open ? t("listPane.collapseHint") : t("listPane.expandHint")}
        onMouseDown={(e) => {
          if (e.detail <= 1) openAtPress.current = open;
        }}
        // A pointer click on the open list waits for the double click; the keyboard (click count 0) folds at once.
        onClick={(e) => {
          if (!open || e.detail === 0) onToggle();
        }}
        onDoubleClick={() => {
          if (open && openAtPress.current) onToggle();
        }}
      />
    </div>
  );
}
