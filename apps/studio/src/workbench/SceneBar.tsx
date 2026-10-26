import { memo } from "react";
import { IconButton } from "../ui/primitives";
import styles from "./SceneBar.module.css";

export interface SceneBarProps {
  showPermissions: boolean;
  showEvents: boolean;
  /** The view looks straight down. */
  topDown: boolean;
  onShowPermissions: (show: boolean) => void;
  onShowEvents: (show: boolean) => void;
  onTopDown: (on: boolean) => void;
}

/** The bar that floats over the 3D view: which layers of the scene are shown and, in a group of its own, whether it looks straight down. */
export const SceneBar = memo(function SceneBar({ showPermissions, showEvents, topDown, onShowPermissions, onShowEvents, onTopDown }: SceneBarProps) {
  return (
    <div role="toolbar" aria-label="Ebenen der Schau" className={styles.bar}>
      <div className={styles.layers}>
        <IconButton label="Gelände" icon="layers" pressed={showPermissions} onClick={() => onShowPermissions(!showPermissions)} />
        <IconButton label="Ereignisse" icon="pin" pressed={showEvents} onClick={() => onShowEvents(!showEvents)} />
      </div>
      <div className={styles.views}>
        <IconButton label="Draufsicht" icon="map" pressed={topDown} onClick={() => onTopDown(!topDown)} />
      </div>
    </div>
  );
});
