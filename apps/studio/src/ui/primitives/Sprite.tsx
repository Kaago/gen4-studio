import styles from "./Sprite.module.css";

export type SpriteProps = {
  /** A ready picture (a blob URL from the icon cache), or `null` while there is none. */
  url: string | null;
  /** What the picture shows; its text alternative, and the placeholder's letter. */
  label: string;
  size: 24 | 32 | 48;
  /** The area whose colours the placeholder takes. */
  area: "pokemon" | "gegenstaende";
  /** Beside text that already names it (a list row): hidden from assistive technology, so the name is not read twice. */
  decorative?: boolean;
};

const sizeClass = { 24: styles.s24, 32: styles.s32, 48: styles.s48 } as const;

/** A game picture drawn pixel for pixel; without one, the first letter of the label on the area's colour. */
export function Sprite({ url, label, size, area, decorative = false }: SpriteProps) {
  if (url !== null) {
    return (
      <img
        className={`${styles.sprite} ${sizeClass[size]}`}
        src={url}
        alt={decorative ? "" : label}
        width={size}
        height={size}
        data-sprite="picture"
      />
    );
  }
  return (
    <span
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : label}
      aria-hidden={decorative ? true : undefined}
      className={`${styles.sprite} ${styles.placeholder} ${styles[area]} ${sizeClass[size]}`}
      data-sprite="placeholder"
      data-area={area}
    >
      <span aria-hidden="true">{label.trim().charAt(0).toUpperCase()}</span>
    </span>
  );
}
