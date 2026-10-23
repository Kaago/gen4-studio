import type { ReactNode } from "react";
import styles from "./PanelHead.module.css";

export type PanelHeadProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
};

/** The own head of a panel whose dock group hides the tab strip. */
export function PanelHead({ title, subtitle, actions }: PanelHeadProps) {
  return (
    <header className={styles.head}>
      <h2 className={styles.title}>{title}</h2>
      {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
      {actions && <div className={styles.actions}>{actions}</div>}
    </header>
  );
}
