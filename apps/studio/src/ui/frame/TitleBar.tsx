import { useTranslation } from "react-i18next";
import { isMacTauri } from "../../platform/os";
import { Icon, IconButton, Kbd } from "../primitives";
import styles from "./TitleBar.module.css";

/** Room for the macOS traffic lights when the title bar overlays the window content. */
const MAC_INSET = 78;

export type TitleBarProps = {
  /** The folder name of the open project (null: none). */
  projectName: string | null;
  /** "Platin, DE" (null: no project). */
  game: string | null;
  dirtyCount: number;
  busy: boolean;
  onOpen: () => void;
  onSave: () => void;
  onBuild: () => void;
  /** Opens the wizard „Veröffentlichen“. */
  onPublish: () => void;
  /** Why „Veröffentlichen“ waits (no found base ROM): shown as hint, the button is aria-disabled. Null: it can open. */
  publishLocked: string | null;
  onHelp: () => void;
  /** Opens the command palette (also ⌘K). */
  onPalette: () => void;
  /** The palette cannot open now (a wizard is open). */
  paletteDisabled?: boolean;
  /** The theme that is shown; the button switches to the other one. */
  theme: "hell" | "dunkel";
  onToggleTheme: () => void;
};

/**
 * The 40 px bar on top. Its row (and every text in it, since Tauri drags only by the element that carries the attribute)
 * is a drag region; the buttons are not.
 */
export function TitleBar({ projectName, game, dirtyCount, busy, onOpen, onSave, onBuild, onPublish, publishLocked, onHelp, onPalette, paletteDisabled = false, theme, onToggleTheme }: TitleBarProps) {
  const { t } = useTranslation("ui");
  const open = projectName !== null;
  const inset = isMacTauri();
  return (
    <div className={styles.bar} data-tauri-drag-region="" style={inset ? { paddingLeft: MAC_INSET } : undefined /* dyn */}>
      <div className={styles.left} data-tauri-drag-region="">
        <IconButton label={t("titleBar.openProject")} icon="open" disabled={busy} onClick={onOpen} />
        <span className={open ? styles.name : styles.noName} data-tauri-drag-region="">
          {projectName ?? t("titleBar.noProject")}
        </span>
        {game !== null && (
          <span className={styles.game} data-tauri-drag-region="">
            {game}
          </span>
        )}
        {open &&
          (dirtyCount > 0 ? (
            <span className={`${styles.saveState} ${styles.unsaved}`} data-tauri-drag-region="">
              <span aria-hidden="true">●</span> {t("titleBar.unsaved", { count: dirtyCount })}
            </span>
          ) : (
            <span className={`${styles.saveState} ${styles.saved}`} data-tauri-drag-region="">
              <Icon name="check" /> {t("titleBar.allSaved")}
            </span>
          ))}
      </div>
      <div className={styles.middle} data-tauri-drag-region="">
        <button type="button" className={styles.palette} disabled={paletteDisabled} aria-keyshortcuts="Meta+K" onClick={onPalette}>
          <Icon name="search" />
          <span className={styles.paletteText}>{t("titleBar.search")}</span>
          <Kbd keys="⌘K" />
        </button>
      </div>
      <div className={styles.right} data-tauri-drag-region="">
        <IconButton label={t("titleBar.save")} icon="save" shortcut="⌘S" disabled={!open} busy={busy} onClick={onSave} />
        <IconButton label={t("titleBar.build")} icon="build" disabled={!open} busy={busy} onClick={onBuild} />
        <IconButton label={t("titleBar.publish")} icon="publish" disabled={!open} busy={busy || publishLocked !== null} hint={open ? (publishLocked ?? undefined) : undefined} onClick={onPublish} />
        <IconButton label={t("titleBar.theme")} icon={theme === "hell" ? "moon" : "sun"} onClick={onToggleTheme} />
        <IconButton label={t("titleBar.help")} icon="help" onClick={onHelp} />
      </div>
    </div>
  );
}
