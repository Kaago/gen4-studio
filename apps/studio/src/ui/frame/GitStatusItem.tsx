import { useTranslation } from "react-i18next";
import type { GitStatus } from "../../api";
import { statusText, statusTooltip } from "../../git/gitWords";
import { Button, Tooltip } from "../primitives";
import styles from "./GitStatusItem.module.css";

export type GitStatusItemProps = {
  /** The Git state as last read; null before it answered (no item then). */
  status: GitStatus | null;
  /** The click: Technik › Änderungen against the last commit. */
  onOpen: () => void;
  /** Why the status could not be read; the item says „Git: nicht lesbar“ with the reason as its tooltip. */
  error?: string | null;
};

/**
 * The status line's Git item: the `git` symbol and „main · 3 Änderungen“ (or „noch kein Commit“, „4c1d2e3 (losgelöst)“), with the count
 * and the last commit as tooltip; a click shows the changes. Nothing without a repository. The accessible name starts with „Git: “ (the
 * symbol is hidden, the tooltip is not linked) and holds the visible text.
 */
export function GitStatusItem({ status, onOpen, error = null }: GitStatusItemProps) {
  const { t } = useTranslation("ui");
  if (error !== null) {
    return (
      <span className={`${styles.item} ${styles.unreadable}`}>
        <Tooltip content={error}>
          <Button kind="quiet" size="s" icon="git" onClick={onOpen} data-git-status="" data-git-error="">
            {t("gitStatusItem.unreadable")}
          </Button>
        </Tooltip>
      </span>
    );
  }
  const text = status === null ? null : statusText(status);
  if (status === null || text === null) return null;
  return (
    <span className={styles.item}>
      <Tooltip content={statusTooltip(status)}>
        <Button kind="quiet" size="s" icon="git" onClick={onOpen} aria-label={t("gitStatusItem.label", { text })} data-git-status="">
          {text}
        </Button>
      </Tooltip>
    </span>
  );
}
