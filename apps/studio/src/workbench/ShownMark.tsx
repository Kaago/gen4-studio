import { Icon } from "../ui/primitives";
import { termLabel } from "./terms";
import styles from "./WorldExplorer.module.css";

/** The mark of a header that uses the matrix the view shows: a symbol with a name, so it does not rest on colour. */
export function ShownMark() {
  return (
    <span className={styles.shown} role="img" aria-label={`${termLabel("matrix")} wird angezeigt`}>
      <Icon name="matrix" />
    </span>
  );
}
