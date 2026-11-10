import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { MelonDsConfigDto, MelonDsDto } from "../../api.live";
import styles from "./LiveRow.module.css";

export type LiveRowProps = {
  /** The melonDS of the last „Hier spielen“ (`live_status.melonds`); null or undefined while none is known. */
  melonds: MelonDsDto | null | undefined;
  /** The chosen emulator (as the emulator row above shows it): while melonDS is not read yet, one chosen is „not checked yet“, not „none“. */
  emulator?: string | null;
};

/**
 * What is missing for a usable stub, each as one sentence that says what the next „Hier spielen“ does about it; empty when the stub is
 * ready. Read from the catalog in the language that is shown.
 */
function missing(config: MelonDsConfigDto, t: TFunction<"ui">): string[] {
  const out: string[] = [];
  if (!config.enabled) out.push(t("liveRow.stubOff"));
  if (config.break_on_startup) out.push(t("liveRow.breakOnStartup"));
  if (config.jit) out.push(t("liveRow.jit"));
  return out;
}

/** The one line of the state: version, stub, port, JIT („melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus“). */
function summary(m: MelonDsDto, t: TFunction<"ui">): string {
  const c = m.config;
  if (c === null) return t("liveRow.settingsUnreadable", { version: m.version });
  const onOff = (on: boolean) => (on ? t("liveRow.on") : t("liveRow.off"));
  return t("liveRow.summary", { version: m.version, stub: onOff(c.enabled), port: c.port, jit: onOff(c.jit) });
}

/**
 * „Spielmodus live“ (Technik › Emulator): whether the melonDS of the last „Hier spielen“ opens the GDB stub the studio connects to — its
 * version, the stub, its port and the JIT, or what is missing and that the next „Hier spielen“ sets it — and where its settings live. Before
 * the first „Hier spielen“ it says that the next one reads them (with an emulator chosen) or that there is no melonDS.
 */
export function LiveRow({ melonds, emulator }: LiveRowProps) {
  const { t } = useTranslation("ui");
  const chosen = emulator !== null && emulator !== undefined && emulator !== "";
  return (
    <div className={styles.row} data-live-row="">
      <span className={styles.label}>{t("liveRow.title")}</span>
      {melonds === null || melonds === undefined ? (
        // The hub reads melonDS at „Hier spielen“.
        <span className={styles.quiet}>{chosen ? t("liveRow.notChecked") : t("liveRow.noMelonDs")}</span>
      ) : (
        <>
          <span className={styles.state}>{summary(melonds, t)}</span>
          {melonds.config !== null &&
            missing(melonds.config, t).map((line) => (
              <span key={line} className={styles.quiet}>
                {line}
              </span>
            ))}
          <span className={styles.path} title={melonds.config_path}>
            {melonds.config_path}
          </span>
        </>
      )}
    </div>
  );
}
