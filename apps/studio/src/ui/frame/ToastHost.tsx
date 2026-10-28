import { useSyncExternalStore } from "react";
import { Button, Icon, IconButton } from "../primitives";
import { listToasts, pauseToast, resumeToast, subscribeToasts, toast } from "./toasts";
import type { Toast } from "./toasts";
import styles from "./ToastHost.module.css";

/** The look and symbol of each kind: progress looks like info. */
const LOOK = { info: "info", progress: "info", warning: "warning", error: "error" } as const;

function ToastItem({ item }: { item: Toast }) {
  const isError = item.kind === "error";
  const look = LOOK[item.kind];
  return (
    <div
      role={isError ? "alert" : "status"}
      className={`${styles.toast} ${styles[look]}`}
      onMouseEnter={() => pauseToast(item.id)}
      onMouseLeave={() => resumeToast(item.id)}
      onFocus={() => pauseToast(item.id)}
      onBlur={() => resumeToast(item.id)}
    >
      <span className={styles.icon}>
        <Icon name={look} weight="fill" />
      </span>
      <div className={styles.body}>
        <p className={styles.text}>{item.text}</p>
        {item.actions.length > 0 && (
          <div className={styles.actions}>
            {item.actions.map((action) => (
              <Button
                key={action.label}
                kind="secondary"
                size="s"
                onClick={() => {
                  action.onClick();
                  toast.dismiss(item.id);
                }}
              >
                {action.label}
              </Button>
            ))}
          </div>
        )}
      </div>
      <IconButton label="Meldung schließen" icon="close" onClick={() => toast.dismiss(item.id)} />
    </div>
  );
}

/** Bottom right above the status bar: at most four toasts from the store, newest last. */
export function ToastHost() {
  const items = useSyncExternalStore(subscribeToasts, listToasts);
  return (
    <section className={styles.host} aria-live="polite" aria-label="Meldungen">
      {items.map((item) => (
        <ToastItem key={item.id} item={item} />
      ))}
    </section>
  );
}
