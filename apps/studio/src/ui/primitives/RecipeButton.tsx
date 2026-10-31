import { useId } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import type { Area } from "./Reference";
import { Tooltip } from "./Tooltip";
import styles from "./RecipeButton.module.css";

export type RecipeButtonProps = {
  /** The recipe's id (`create_room`), as `data-recipe`. */
  id: string;
  /** The visible name and the accessible name („Raum anlegen“). */
  label: string;
  icon: IconName;
  /** The area whose colour the symbol's tile carries, as on a `Reference`. */
  area: Area;
  /** Second line of the tooltip: what the recipe makes. */
  hint?: string;
  pressed?: boolean;
  /** Why the recipe cannot run now; the button is disabled and the tooltip says it. */
  reason?: string | null;
  onClick: () => void;
  size?: "m" | "s";
};

/**
 * A recipe as a key: the symbol on a tile in its area's colour, the name beside it (a recipe is a word, never only a letter). Rest, hover,
 * pressed (cobalt edge, `aria-pressed`) and locked: disabled, the name dimmed and a lock after it (never colour alone), the reason in the
 * tooltip and as the description. A locked key cannot take focus, so its wrapper takes the pointer for the tooltip.
 */
export function RecipeButton({ id, label, icon, area, hint, pressed, reason = null, onClick, size = "m" }: RecipeButtonProps) {
  const reasonId = useId();
  const locked = reason !== null;
  const key = (
    <button
      type="button"
      className={styles.recipe}
      data-recipe={id}
      data-area={area}
      data-size={size}
      data-locked={locked ? "" : undefined}
      // Only a recipe that runs takes part in a bar's roving tab stop.
      data-roving={locked ? undefined : ""}
      aria-pressed={pressed}
      aria-describedby={locked ? reasonId : undefined}
      disabled={locked}
      onClick={onClick}
    >
      <span className={`${styles.tile} ${styles[area]}`}>
        <Icon name={icon} weight={pressed ? "fill" : "regular"} />
      </span>
      <span className={styles.label}>{label}</span>
      {locked && (
        <span className={styles.lock}>
          <Icon name="lock" />
        </span>
      )}
    </button>
  );
  const tip = (
    <span className={styles.tip}>
      <span className={styles.tipHead}>{label}</span>
      {(reason ?? hint) && <span className={styles.tipHint}>{reason ?? hint}</span>}
    </span>
  );
  if (!locked) return <Tooltip content={tip}>{key}</Tooltip>;
  return (
    <>
      <Tooltip content={tip}>
        <span className={styles.lockedWrap}>{key}</span>
      </Tooltip>
      <span id={reasonId} hidden>
        {reason}
      </span>
    </>
  );
}
