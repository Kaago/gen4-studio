import type { ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./Badge.module.css";

export type BadgeKind = "unsaved" | "irreversible" | "locked" | "changedOnDisk" | "ptHgssOnly" | "error" | "live" | "info";

const defaults: Record<BadgeKind, { text: string; icon: IconName }> = {
  unsaved: { text: "Ungespeichert", icon: "unsaved" },
  irreversible: { text: "Nicht rückgängig", icon: "warning" },
  locked: { text: "Gesperrt", icon: "lock" },
  changedOnDisk: { text: "Auf Platte geändert", icon: "history" },
  ptHgssOnly: { text: "Nur Platin und HGSS", icon: "info" },
  error: { text: "Fehler", icon: "error" },
  live: { text: "Live", icon: "play" },
  info: { text: "Info", icon: "info" },
};

export type BadgeProps = {
  kind: BadgeKind;
  /** Replaces the default text of the kind. */
  children?: ReactNode;
};

export function Badge({ kind, children }: BadgeProps) {
  const { text, icon } = defaults[kind];
  return (
    <span className={`${styles.badge} ${styles[kind]}`}>
      <Icon name={icon} weight="fill" />
      {children ?? text}
    </span>
  );
}
