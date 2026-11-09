import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./Badge.module.css";

export type BadgeKind =
  | "unsaved"
  | "irreversible"
  | "locked"
  | "changedOnDisk"
  | "ptHgssOnly"
  | "error"
  | "live"
  | "info"
  | "success"
  | "warning"
  | "code"
  | "neutral";

/** The symbol of each kind; its default text is `badge.<kind>` in the `ui` catalog. */
const icons: Record<BadgeKind, IconName | undefined> = {
  unsaved: "unsaved",
  irreversible: "warning",
  locked: "lock",
  changedOnDisk: "history",
  ptHgssOnly: "info",
  error: "error",
  live: "play",
  info: "info",
  success: "check",
  warning: "warning",
  code: undefined,
  neutral: undefined,
};

export type BadgeProps = {
  kind: BadgeKind;
  /** Replaces the default text of the kind. */
  children?: ReactNode;
  /** Native tooltip; the `code` badge for an unknown code names the code here. */
  title?: string;
  /** Replaces the symbol of the kind (or gives one to a kind without, like the game chip on `neutral`). */
  icon?: IconName;
};

export function Badge({ kind, children, title, icon: given }: BadgeProps) {
  const { t } = useTranslation("ui");
  const icon = given ?? icons[kind];
  return (
    <span className={`${styles.badge} ${styles[kind]}`} title={title}>
      {icon && <Icon name={icon} weight="fill" />}
      {children ?? t(`badge.${kind}`)}
    </span>
  );
}
