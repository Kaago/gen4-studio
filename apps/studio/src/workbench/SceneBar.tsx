import { memo } from "react";
import { IconButton } from "../ui/primitives";
import styles from "./SceneBar.module.css";

export interface SceneBarProps {
  showPermissions: boolean;
  showEvents: boolean;
  onShowPermissions: (show: boolean) => void;
  onShowEvents: (show: boolean) => void;
}

/** The bar that floats over the 3D view: which layers of the scene are shown. */
export const SceneBar = memo(function SceneBar({ showPermissions, showEvents, onShowPermissions, onShowEvents }: SceneBarProps) {
  return (
    <div role="toolbar" aria-label="Ebenen der Schau" className={styles.bar}>
      <div className={styles.layers}>
        <IconButton label="Gelände" icon="layers" pressed={showPermissions} onClick={() => onShowPermissions(!showPermissions)} />
        <IconButton label="Ereignisse" icon="pin" pressed={showEvents} onClick={() => onShowEvents(!showEvents)} />
      </div>
    </div>
  );
});
