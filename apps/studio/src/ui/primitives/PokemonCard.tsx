import type { KeyboardEvent } from "react";
import type { RowDataAttrs } from "./ListRow";
import { Sprite } from "./Sprite";
import styles from "./PokemonCard.module.css";

export type PokemonCardProps = {
  /** `selected`: the place whose details show (Kobalt edge); `empty`: a free place of the team (dashed, "Platz 4 leer"). */
  state: "idle" | "selected" | "empty";
  /** The species shown (`data-species`); absent on an empty place. */
  species?: number;
  name?: string;
  level?: number;
  /** The held item in words; without it the card has no item line. */
  item?: string;
  /** The menu icon (a blob URL from the icon cache); null or absent: the first letter of the name. */
  iconUrl?: string | null;
  onClick: () => void;
  /** The card's accessible name ("Platz 1: Staralili, Level 12"). */
  ariaLabel: string;
  /** The 1-based place in the team, for the empty card's text. */
  slot?: number;
  /** Shows but does not react (a locked trainer). */
  disabled?: boolean;
  /** Pass-through `data-*` attributes (`data-slot`) that tests and scripts address the card by. */
  dataAttrs?: RowDataAttrs;
};

/**
 * One place of a trainer's team as a card on the sheet surface: the menu icon, the name, "Lv. 12" and the held item. The chosen place
 * carries the Kobalt edge (`aria-pressed`); a free place is dashed and says so. A button: click, Enter and Space call `onClick`.
 */
export function PokemonCard({ state, species, name, level, item, iconUrl, onClick, ariaLabel, slot, disabled, dataAttrs }: PokemonCardProps) {
  const empty = state === "empty";
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (disabled || event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
    event.preventDefault();
    onClick();
  };
  return (
    <div
      {...dataAttrs}
      role="button"
      tabIndex={disabled ? -1 : 0}
      aria-label={ariaLabel}
      aria-pressed={empty ? undefined : state === "selected"}
      aria-disabled={disabled ? true : undefined}
      data-state={state}
      data-species={species}
      className={[styles.card, styles[state], disabled ? styles.disabled : ""].filter(Boolean).join(" ")}
      onClick={disabled ? undefined : onClick}
      onKeyDown={onKeyDown}
    >
      {empty ? (
        <span className={styles.emptyText}>{slot === undefined ? "Leer" : `Platz ${slot} leer`}</span>
      ) : (
        <>
          <Sprite url={iconUrl ?? null} label={name ?? ""} size={32} area="pokemon" decorative />
          <span className={styles.text}>
            <span className={styles.name}>{name}</span>
            {level !== undefined && <span className={styles.level}>{`Lv. ${level}`}</span>}
            {item && <span className={styles.item}>{item}</span>}
          </span>
        </>
      )}
    </div>
  );
}
