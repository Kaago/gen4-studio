import { useTranslation } from "react-i18next";
import { pokemonType, typeName } from "./pokemonTypes";
import styles from "./TypeBadge.module.css";

export type TypeBadgeProps = {
  /** The type number from the game data (0 Normal … 17 Unlicht). */
  type: number;
  /** `s` for lists and table cells, `m` (default) for heads and cards. */
  size?: "s" | "m";
};

/** The colour class of a type (background and text from its `szene/typ` tokens); an unknown type gets a neutral look. */
export function typeClass(type: number): string {
  const known = pokemonType(type);
  return known ? styles[known.key]! : styles.unknown!;
}

/** A Pokémon type: its colour and, never colour alone, its name. */
export function TypeBadge({ type, size = "m" }: TypeBadgeProps) {
  // `typeName` reads the catalog; subscribing makes the badge render again when the language changes.
  useTranslation("ui");
  const unknown = pokemonType(type) === undefined;
  return (
    <span className={`${styles.badge} ${styles[size]} ${typeClass(type)}`} data-type={type} data-unknown={unknown ? "" : undefined}>
      {typeName(type)}
    </span>
  );
}
