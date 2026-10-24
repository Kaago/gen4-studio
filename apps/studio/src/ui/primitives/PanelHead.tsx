import type { ReactNode } from "react";
import styles from "./PanelHead.module.css";

export type PanelHeadProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  /** A state shown at the right end (a badge); in a narrow head it stays beside the title and the actions take their own row. */
  status?: ReactNode;
};

/** The own head of a panel whose dock group hides the tab strip. */
export function PanelHead({ title, subtitle, actions, status }: PanelHeadProps) {
  return (
    <header className={status ? `${styles.head} ${styles.withStatus}` : styles.head}>
      <h2 className={styles.title}>{title}</h2>
      {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
      {actions && <div className={styles.actions}>{actions}</div>}
      {status && (
        <div className={styles.status} data-panel-status="">
          {status}
        </div>
      )}
    </header>
  );
}
