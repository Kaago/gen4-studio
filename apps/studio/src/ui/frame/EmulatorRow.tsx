import { useTranslation } from "react-i18next";
import { Button } from "../primitives";
import styles from "./EmulatorRow.module.css";

export type EmulatorRowProps = {
  /** The chosen emulator; null or undefined while none is chosen. */
  path: string | null | undefined;
  /** Asks for an emulator (the system's file dialog). */
  onPick: () => void;
};

/** The emulator the studio starts a built ROM in, and the way to choose another one. Shared by Einstellungen and Technik. */
export function EmulatorRow({ path, onPick }: EmulatorRowProps) {
  const { t } = useTranslation("ui");
  const chosen = path !== null && path !== undefined && path !== "";
  return (
    <div className={styles.row}>
      {chosen ? (
        <span className={styles.path} title={path}>
          {path}
        </span>
      ) : (
        <span className={styles.none}>{t("emulatorRow.none")}</span>
      )}
      <Button kind="secondary" size="s" onClick={onPick}>
        {t("emulatorRow.pick")}
      </Button>
    </div>
  );
}
