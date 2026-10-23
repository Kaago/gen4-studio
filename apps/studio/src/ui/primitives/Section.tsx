import { useId } from "react";
import type { ReactNode } from "react";
import { Icon } from "./Icon";
import styles from "./Section.module.css";

export type SectionProps = {
  title: string;
  count?: number;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
};

/** A collapsible group with its own head: the head is a button, the open content is a region named by it. */
export function Section({ title, count, open, onToggle, children }: SectionProps) {
  const headId = useId();
  const bodyId = useId();
  return (
    <section className={styles.section}>
      <h3 className={styles.heading}>
        <button type="button" id={headId} className={styles.head} aria-expanded={open} aria-controls={bodyId} onClick={onToggle}>
          <Icon name={open ? "caretDown" : "caretRight"} />
          <span className={styles.title}>{title}</span>
          {count !== undefined && <span className={styles.count}>{count}</span>}
        </button>
      </h3>
      {open && (
        <div id={bodyId} role="region" aria-labelledby={headId} className={styles.body}>
          {children}
        </div>
      )}
    </section>
  );
}
