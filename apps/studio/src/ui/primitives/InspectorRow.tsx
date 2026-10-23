import type { ReactNode } from "react";
import styles from "./InspectorRow.module.css";

export type InspectorRowProps = {
  label: string;
  /** The value differs from the vanilla or default value: a 2 px accent bar on the left, named in text for assistive technology. */
  deviates?: boolean;
  hint?: string;
  children: ReactNode;
};

/** One labelled field of the inspector: a 116 px label column and the control. */
export function InspectorRow({ label, deviates, hint, children }: InspectorRowProps) {
  return (
    <div className={`${styles.row} ${deviates ? styles.deviates : ""}`}>
      <span className={styles.label}>
        {label}
        {deviates && <span className={styles.hidden}> weicht ab</span>}
      </span>
      <div className={styles.control}>{children}</div>
      {hint && <p className={styles.hint}>{hint}</p>}
    </div>
  );
}
