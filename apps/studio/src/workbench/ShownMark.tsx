import { useTranslation } from "react-i18next";
import { Icon } from "../ui/primitives";
import { termLabel } from "./terms";
import styles from "./WorldExplorer.module.css";

/** The mark of a header that uses the matrix the view shows: a symbol with a name, so it does not rest on colour. */
export function ShownMark() {
  const { t } = useTranslation("workbench");
  return (
    <span className={styles.shown} role="img" aria-label={t("shownMark.label", { matrix: termLabel("matrix") })}>
      <Icon name="matrix" />
    </span>
  );
}
