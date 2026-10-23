import type { ReactNode } from "react";
import { IconButton } from "../ui/primitives";
import styles from "./SceneBar.module.css";

export interface SceneBarProps {
  showPermissions: boolean;
  showEvents: boolean;
  onShowPermissions: (show: boolean) => void;
  onShowEvents: (show: boolean) => void;
  /** More controls after the layer toggles (until the tools of the hand take them over). */
  extra?: ReactNode;
}

/** The bar that floats over the 3D view: which layers of the scene are shown. */
export function SceneBar({ showPermissions, showEvents, onShowPermissions, onShowEvents, extra }: SceneBarProps) {
  return (
    <div role="toolbar" aria-label="Ebenen der Schau" className={styles.bar}>
      <div className={styles.layers}>
        <IconButton label="Gelände" icon="layers" pressed={showPermissions} onClick={() => onShowPermissions(!showPermissions)} />
        <IconButton label="Ereignisse" icon="pin" pressed={showEvents} onClick={() => onShowEvents(!showEvents)} />
      </div>
      {extra && <div className={styles.extra}>{extra}</div>}
    </div>
  );
}
