import { useId } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import styles from "./StatBar.module.css";

export type StatBarProps = {
  label: string;
  value: number;
  /** The end of the scale; base stats go to 255. */
  max?: number;
  /** The average to compare with (e.g. of all species); drawn as a stroke and named in text. */
  average: number;
  /** The value differs from the original: an accent bar on the left, named in text for assistive technology. */
  deviates?: boolean;
};

/** Where a value stands against the average; within half a point counts as on it. */
function verdict(value: number, average: number, t: TFunction<"ui">): string {
  if (value >= average + 0.5) return t("statBar.above");
  if (value <= average - 0.5) return t("statBar.below");
  return t("statBar.at");
}

const percent = (part: number, max: number) => `${(Math.min(Math.max(part, 0), max) / max) * 100}%`;

/** One stat as a bar up to `max` with a stroke at the average, and in words where it stands. */
export function StatBar({ label, value, max = 255, average, deviates }: StatBarProps) {
  const { t } = useTranslation("ui");
  const labelId = `${useId()}-label`;
  const said = verdict(value, average, t);
  return (
    <div className={`${styles.row} ${deviates ? styles.deviates : ""}`} data-deviates={deviates ? "" : undefined}>
      <span id={labelId} className={styles.label}>
        {label}
        {deviates && <span className={styles.hidden}> {t("statBar.deviates")}</span>}
      </span>
      <span className={styles.value}>{value}</span>
      <div
        role="meter"
        aria-labelledby={labelId}
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuetext={`${value}, ${said}`}
        className={styles.track}
      >
        <span className={styles.bar} style={{ width: percent(value, max) } /* dyn */} />
        <span className={styles.average} style={{ left: percent(average, max) } /* dyn */} />
      </div>
      <span className={styles.verdict}>{said}</span>
    </div>
  );
}
