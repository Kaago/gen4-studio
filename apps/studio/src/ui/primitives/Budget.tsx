import { useId } from "react";
import { Badge } from "./Badge";
import styles from "./Budget.module.css";

export type BudgetState = "calm" | "tight" | "full";

export type BudgetProps = {
  label: string;
  used: number;
  max: number;
  /** Ruhig (no badge), Knapp (a warning badge) or Voll (an error badge); the owner decides, the bar only shows it. */
  state: BudgetState;
  /** Unit after the numbers („B“); none for a count. */
  unit?: string;
  /** A second line in small print („Vanilla höchstens 110.640 B“). */
  note?: string;
  /** Replaces the words „<used> von <max> <unit>“ (they also name the meter's value, with the true number over the limit). */
  text?: string;
  /** The limit for Tüfteln (hex, „0xF000“), grey in mono after the words. */
  tech?: string;
};

const BADGES: Record<Exclude<BudgetState, "calm">, { kind: "warning" | "error"; text: string }> = {
  tight: { kind: "warning", text: "Knapp" },
  full: { kind: "error", text: "Voll" },
};

const number = (n: number) => n.toLocaleString("de-DE");

/** The share of `max` the bar fills (a full bar over the limit, an empty one without a limit). */
const share = (used: number, max: number) => `${max > 0 ? (Math.min(Math.max(used, 0), max) / max) * 100 : 0}%`;

/**
 * One limit of the game, calmly: the name, „<used> von <max>“ and a bar (`role="meter"`); Knapp and Voll add their badge and colour the
 * bar, Ruhig stays quiet. A budget never blocks saving; it only says where the game's limits are.
 */
export function Budget({ label, used, max, state, unit, note, text, tech }: BudgetProps) {
  const labelId = `${useId()}-label`;
  const words = text ?? `${number(used)} von ${number(max)}${unit ? ` ${unit}` : ""}`;
  const badge = state === "calm" ? null : BADGES[state];
  return (
    <div className={`${styles.budget} ${styles[state]}`} data-budget-state={state}>
      <div className={styles.head}>
        <span id={labelId} className={styles.label}>
          {label}
        </span>
        {badge && <Badge kind={badge.kind}>{badge.text}</Badge>}
      </div>
      <div
        role="meter"
        aria-labelledby={labelId}
        aria-valuenow={Math.min(Math.max(used, 0), max)}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuetext={badge ? `${words}, ${badge.text}` : words}
        className={styles.track}
      >
        <span className={styles.fill} data-budget-fill="" style={{ width: share(used, max) } /* dyn */} />
      </div>
      <div className={styles.value}>
        <span>{words}</span>
        {tech && (
          <span className={styles.tech} data-tech="">
            {tech}
          </span>
        )}
      </div>
      {note && <div className={styles.note}>{note}</div>}
    </div>
  );
}
