import { Button } from "./Button";
import styles from "./EmptyState.module.css";

export type EmptyStateProps = {
  /** What is missing. */
  what: string;
  /** Why, and the way out. */
  why: string;
  action?: { label: string; onClick: () => void };
};

export function EmptyState({ what, why, action }: EmptyStateProps) {
  return (
    <div className={styles.empty}>
      <p className={styles.what}>{what}</p>
      <p className={styles.why}>{why}</p>
      {action && (
        <Button kind="secondary" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
