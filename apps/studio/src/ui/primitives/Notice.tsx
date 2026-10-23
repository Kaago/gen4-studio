import type { ReactNode } from "react";
import { Button } from "./Button";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./Notice.module.css";

export type NoticeKind = "info" | "success" | "warning" | "error";

const symbol: Record<NoticeKind, IconName> = { info: "info", success: "success", warning: "warning", error: "error" };

export type NoticeProps = {
  kind: NoticeKind;
  title?: string;
  actions?: { label: string; onClick: () => void }[];
  children: ReactNode;
};

/** An inline message. Errors are announced at once (`role="alert"`), the rest politely (`role="status"`). */
export function Notice({ kind, title, actions, children }: NoticeProps) {
  return (
    <div role={kind === "error" ? "alert" : "status"} className={`${styles.notice} ${styles[kind]}`}>
      <span className={styles.icon}>
        <Icon name={symbol[kind]} weight="fill" />
      </span>
      <div className={styles.body}>
        {title && <p className={styles.title}>{title}</p>}
        <div className={styles.text}>{children}</div>
        {actions && actions.length > 0 && (
          <div className={styles.actions}>
            {actions.map((a) => (
              <Button key={a.label} kind="secondary" size="s" onClick={a.onClick}>
                {a.label}
              </Button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
