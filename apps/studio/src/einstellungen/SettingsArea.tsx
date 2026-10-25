import { Kbd, Segmented } from "../ui/primitives";
import type { SegmentedOption } from "../ui/primitives";
import { EmulatorRow } from "../ui/frame/EmulatorRow";
import type { Density, Theme } from "../ui/tokens/theme";
import { SHORTCUTS } from "../world/buildings";
import { AgentsSettings } from "./AgentsSettings";
import type { AgentsSettingsProps } from "./AgentsSettings";
import styles from "./SettingsArea.module.css";

export type SettingsAreaProps = {
  /** The theme as chosen ("system" follows the OS). */
  theme: Theme;
  density: Density;
  /** The chosen emulator; undefined or null while none is chosen. */
  emulatorPath: string | null | undefined;
  onTheme: (t: Theme) => void;
  onDensity: (d: Density) => void;
  onPickEmulator: () => void;
  /** The column Agenten (local MCP clients in the running app). */
  agents: AgentsSettingsProps;
};

const THEMES: readonly SegmentedOption<Theme>[] = [
  { value: "hell", label: "Hell", icon: "sun" },
  { value: "dunkel", label: "Dunkel", icon: "moon" },
  { value: "system", label: "Wie System", icon: "system" },
];

const DENSITIES: readonly SegmentedOption<Density>[] = [
  { value: "komfort", label: "Komfort" },
  { value: "kompakt", label: "Kompakt" },
];

/**
 * The surface of the area Einstellungen: appearance, emulator, the app's shortcuts (read only) and the agents. Every change applies at
 * once, for every project (the agents' grant of the open project is the one setting per project).
 */
export function SettingsArea({ theme, density, emulatorPath, onTheme, onDensity, onPickEmulator, agents }: SettingsAreaProps) {
  return (
    <section className={styles.area} aria-labelledby="settings-title">
      <h1 id="settings-title" className={styles.title}>
        Einstellungen
      </h1>
      <div className={styles.columns}>
        <section className={styles.column} aria-labelledby="settings-look">
          <h2 id="settings-look" className={styles.heading}>
            Darstellung
          </h2>
          <div className={styles.field}>
            <span className={styles.label}>Thema</span>
            <Segmented options={THEMES} value={theme} onChange={onTheme} ariaLabel="Thema" />
          </div>
          <div className={styles.field}>
            <span className={styles.label}>Dichte</span>
            <Segmented options={DENSITIES} value={density} onChange={onDensity} ariaLabel="Dichte" />
            <p className={styles.hint}>Kompakt: Zeilen 24 statt 32 px hoch</p>
          </div>
        </section>
        <section className={styles.column} aria-labelledby="settings-emulator">
          <h2 id="settings-emulator" className={styles.heading}>
            Emulator
          </h2>
          <EmulatorRow path={emulatorPath} onPick={onPickEmulator} />
        </section>
        <section className={styles.column} aria-labelledby="settings-keys">
          <h2 id="settings-keys" className={styles.heading}>
            Tastenkürzel
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
