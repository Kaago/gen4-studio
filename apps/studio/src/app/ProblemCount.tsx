import { useSyncExternalStore } from "react";
import { countsText } from "../workbench/problems/problems";
import type { ProblemsStore } from "../workbench/problems/problems";
import styles from "./ProblemCount.module.css";

/** "2 Fehler, 1 Warnung" for the status line; nothing while the project has no problems. Reads the store itself, so only this renders when it changes. */
export function ProblemCount({ store }: { store: ProblemsStore }) {
  const counts = useSyncExternalStore(store.subscribe, store.counts);
  const text = countsText(counts);
  if (text === "") return null;
  return <span className={counts.errors > 0 ? styles.errors : styles.warnings}>{text}</span>;
}
