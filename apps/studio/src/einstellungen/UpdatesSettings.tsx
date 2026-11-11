import { useTranslation } from "react-i18next";
import { Button, Switch } from "../ui/primitives";
import { ago, useNow } from "../ui/frame/timeAgo";
import styles from "./SettingsArea.module.css";

export type UpdatesSettingsProps = {
  /** The installed version („1.0.1“). */
  version: string;
  /** Whether Gen4 Studio looks for updates at startup on its own (consent „auto“). */
  auto: boolean;
  /** The ISO time of the last successful check; null while there was none. */
  lastCheck: string | null;
  /** A check is running: „Jetzt suchen“ waits. */
  checking: boolean;
  onAuto: (on: boolean) => void;
  onCheckNow: () => void;
};

/** The column Updates: the installed version, the switch for the daily check, „Jetzt suchen“ and when it last looked. */
export function UpdatesSettings({ version, auto, lastCheck, checking, onAuto, onCheckNow }: UpdatesSettingsProps) {
  const { t } = useTranslation("app");
  const now = useNow();
  const last = lastCheck === null ? Number.NaN : Date.parse(lastCheck);
  return (
    <section className={styles.column} aria-labelledby="settings-updates">
      <h2 id="settings-updates" className={styles.heading}>
        {t("settings.updates")}
      </h2>
      <div className={styles.field}>
        <span className={styles.label}>{t("settings.updatesVersion", { version })}</span>
      </div>
      <div className={styles.field}>
        <Switch label={t("settings.updatesAuto")} checked={auto} onChange={onAuto} />
        <p className={styles.hint}>{t("settings.updatesHint")}</p>
      </div>
      <div className={styles.field}>
        <Button kind="secondary" disabled={checking} onClick={onCheckNow}>
          {t("settings.updatesCheckNow")}
        </Button>
        <p className={styles.hint}>{Number.isNaN(last) ? t("settings.updatesNeverChecked") : t("settings.updatesLastChecked", { when: ago(last, now) })}</p>
      </div>
    </section>
  );
}
