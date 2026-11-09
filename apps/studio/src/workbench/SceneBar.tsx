import { memo } from "react";
import { useTranslation } from "react-i18next";
import { IconButton } from "../ui/primitives";
import styles from "./SceneBar.module.css";

export interface SceneBarProps {
  showPermissions: boolean;
  /** The layer „Höhen“ is shown; switching it on switches „Gelände“ off and back (the owner keeps them apart). */
  showHeights: boolean;
  showEvents: boolean;
  /** The view looks straight down. */
  topDown: boolean;
  onShowPermissions: (show: boolean) => void;
  onShowHeights: (show: boolean) => void;
  onShowEvents: (show: boolean) => void;
  onTopDown: (on: boolean) => void;
}

/** The bar that floats over the 3D view: which layers of the scene are shown and, in a group of its own, whether it looks straight down. */
export const SceneBar = memo(function SceneBar({ showPermissions, showHeights, showEvents, topDown, onShowPermissions, onShowHeights, onShowEvents, onTopDown }: SceneBarProps) {
  const { t } = useTranslation("workbench");
  return (
    <div role="toolbar" aria-label={t("sceneBar.label")} className={styles.bar}>
      <div className={styles.layers}>
        <IconButton label={t("sceneBar.terrain")} icon="layers" pressed={showPermissions} onClick={() => onShowPermissions(!showPermissions)} />
        <IconButton label={t("sceneBar.heights")} icon="heights" pressed={showHeights} onClick={() => onShowHeights(!showHeights)} />
        <IconButton label={t("sceneBar.events")} icon="pin" pressed={showEvents} onClick={() => onShowEvents(!showEvents)} />
      </div>
      <div className={styles.views}>
        <IconButton label={t("sceneBar.topDown")} icon="topDown" pressed={topDown} onClick={() => onTopDown(!topDown)} />
      </div>
    </div>
  );
});
