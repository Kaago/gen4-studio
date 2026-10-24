import type { ReactNode } from "react";
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

const defaults: Record<BadgeKind, { text: string; icon?: IconName }> = {
  unsaved: { text: "Ungespeichert", icon: "unsaved" },
  irreversible: { text: "Nicht rückgängig", icon: "warning" },
  locked: { text: "Gesperrt", icon: "lock" },
  changedOnDisk: { text: "Auf Platte geändert", icon: "history" },
  ptHgssOnly: { text: "Nur Platin und HGSS", icon: "info" },
  error: { text: "Fehler", icon: "error" },
  live: { text: "Live", icon: "play" },
  info: { text: "Info", icon: "info" },
  success: { text: "Erledigt", icon: "check" },
  warning: { text: "Warnung", icon: "warning" },
  code: { text: "Code" },
  neutral: { text: "Hinweis" },
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
  const { text, icon: fallback } = defaults[kind];
  const icon = given ?? fallback;
  return (
    <span className={`${styles.badge} ${styles[kind]}`} title={title}>
      {icon && <Icon name={icon} weight="fill" />}
      {children ?? text}
    </span>
  );
}
