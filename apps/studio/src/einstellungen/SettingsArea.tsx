import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { Language } from "../i18n";
import { Kbd, Segmented } from "../ui/primitives";
import type { SegmentedOption } from "../ui/primitives";
import { EmulatorRow } from "../ui/frame/EmulatorRow";
import type { Density, Theme } from "../ui/tokens/theme";
import { SHORTCUTS } from "../world/buildings";
import { AgentsSettings } from "./AgentsSettings";
import type { AgentsSettingsProps } from "./AgentsSettings";
import { UpdatesSettings } from "./UpdatesSettings";
import type { UpdatesSettingsProps } from "./UpdatesSettings";
import styles from "./SettingsArea.module.css";

export type SettingsAreaProps = {
  /** The theme as chosen ("system" follows the OS). */
  theme: Theme;
  density: Density;
  /** The language as chosen ("system" follows the OS). */
  language: Language;
  /** The chosen emulator; undefined or null while none is chosen. */
  emulatorPath: string | null | undefined;
  onTheme: (t: Theme) => void;
  onDensity: (d: Density) => void;
  onLanguage: (l: Language) => void;
  /** The language was just switched (which remounted this surface): the pressed language option takes the focus when it mounts. */
  focusLanguage?: boolean;
  onPickEmulator: () => void;
  /** The column Agenten (local MCP clients in the running app). */
  agents: AgentsSettingsProps;
  /** The column Updates, after Emulator; left out where the updater does not run (dev build, browser mode). */
  updates?: UpdatesSettingsProps;
};

/**
 * The surface of the area Einstellungen: appearance, emulator, updates (in the packaged app), the app's shortcuts (read only) and the agents.
 * Every change applies at once, for every project (the agents' grant of the open project is the one setting per project).
 */
export function SettingsArea({ theme, density, language, emulatorPath, onTheme, onDensity, onLanguage, focusLanguage = false, onPickEmulator, agents, updates }: SettingsAreaProps) {
  const { t } = useTranslation("app");
  // Built per render, not at module scope: the labels follow the language the user just switched to.
  const themes: readonly SegmentedOption<Theme>[] = [
    { value: "hell", label: t("settings.themeLight"), icon: "sun" },
    { value: "dunkel", label: t("settings.themeDark"), icon: "moon" },
    { value: "system", label: t("settings.themeSystem"), icon: "system" },
  ];
  const densities: readonly SegmentedOption<Density>[] = [
    { value: "komfort", label: t("settings.densityComfort") },
    { value: "kompakt", label: t("settings.densityCompact") },
  ];
  const languages: readonly SegmentedOption<Language>[] = [
    { value: "system", label: t("settings.languageSystem") },
    { value: "de", label: t("settings.languageDe") },
    { value: "en", label: t("settings.languageEn") },
  ];
  // A switch remounts the app below the frame: the focus, which was on the option just pressed, goes back to it.
  const languageField = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (focusLanguage) languageField.current?.querySelector<HTMLElement>('[aria-pressed="true"]')?.focus();
    // Only when this surface mounts: a later render must not pull the focus.
  }, []);
  return (
    <section className={styles.area} aria-labelledby="settings-title">
      <h1 id="settings-title" className={styles.title}>
        {t("settings.title")}
      </h1>
      <div className={styles.columns}>
        <section className={styles.column} aria-labelledby="settings-look">
          <h2 id="settings-look" className={styles.heading}>
            {t("settings.look")}
          </h2>
          <div className={styles.field}>
            <span className={styles.label}>{t("settings.theme")}</span>
            <Segmented options={themes} value={theme} onChange={onTheme} ariaLabel={t("settings.theme")} />
          </div>
          <div className={styles.field}>
            <span className={styles.label}>{t("settings.density")}</span>
            <Segmented options={densities} value={density} onChange={onDensity} ariaLabel={t("settings.density")} />
            <p className={styles.hint}>{t("settings.densityHint")}</p>
          </div>
          <div className={styles.field} ref={languageField}>
            <span className={styles.label}>{t("settings.language")}</span>
            <Segmented options={languages} value={language} onChange={onLanguage} ariaLabel={t("settings.language")} />
          </div>
        </section>
        <section className={styles.column} aria-labelledby="settings-emulator">
          <h2 id="settings-emulator" className={styles.heading}>
            {t("settings.emulator")}
          </h2>
          <EmulatorRow path={emulatorPath} onPick={onPickEmulator} />
        </section>
        {updates && <UpdatesSettings {...updates} />}
        <section className={styles.column} aria-labelledby="settings-keys">
          <h2 id="settings-keys" className={styles.heading}>
            {t("settings.keys")}
          </h2>
          <dl className={styles.keys}>
            {Object.entries(SHORTCUTS).map(([action, { keys, label }]) => (
              <div key={action} className={styles.key}>
                <dt>{label}</dt>
                <dd>
                  <Kbd keys={keys} />
                </dd>
              </div>
            ))}
          </dl>
        </section>
        <AgentsSettings {...agents} />
      </div>
    </section>
  );
}
