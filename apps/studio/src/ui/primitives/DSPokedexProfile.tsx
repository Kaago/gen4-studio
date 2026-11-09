import { useTranslation } from "react-i18next";
import type { Page } from "../../workbench/text/textModel";
import { DSScreen } from "./DSScreen";
import { Sprite } from "./Sprite";
import { TypeBadge } from "./TypeBadge";
import styles from "./DSPokedexProfile.module.css";

export type DSPokedexProfileProps = {
  /** National Pokédex number, shown with three digits („Nr. 397“). */
  number: number;
  name: string;
  /** One or two type numbers; two equal ones (how the game stores a single type) show one badge. */
  types: readonly [number] | readonly [number, number];
  /** The 80 × 80 front picture as a blob URL; `null` shows the placeholder (the name's first letter). */
  spriteUrl: string | null;
  /** Missing (the Pokédex data is not read yet): „folgt“. */
  category?: string;
  height?: string;
  weight?: string;
  /** Missing: „Pokédex-Text folgt“. */
  text?: string;
  /** Whole-number magnification as in `DSScreen`; measured from the container when missing. */
  scale?: number;
  /** Only decoration, as in `DSScreen`: hidden from assistive technology. */
  decorative?: boolean;
};

const NO_PAGES: Page[] = [];
function Field({ label, value }: { label: string; value: string | undefined }) {
  const { t } = useTranslation("ui");
  return (
    <div className={styles.line}>
      <span>{label}</span>
      <span>{value || t("dsPokedexProfile.pending")}</span>
    </div>
  );
}

/**
 * A Pokédex page on the DS screen: the picture on a pedestal on the left, number, name, types, category, height and weight on the right,
 * the Pokédex text in the box below. Drawn into the background of an empty `DSScreen` (same frame, scale and game font).
 */
export function DSPokedexProfile({ number, name, types, spriteUrl, category, height, weight, text, scale, decorative = false }: DSPokedexProfileProps) {
  const { t } = useTranslation("ui");
  return (
    <DSScreen
      variant="empty"
      pages={NO_PAGES}
      page={0}
      scale={scale}
      decorative={decorative}
      label={t("dsPokedexProfile.label", { name })}
      background={
        <div className={styles.profile} data-ds-profile="">
          <div className={styles.pedestal} />
          <div className={styles.picture}>
            {/* The name stands beside it in text. */}
            <Sprite url={spriteUrl} label={name} size={80} area="pokemon" decorative />
          </div>
          <div className={styles.panel}>
            <div className={styles.line}>{t("dsPokedexProfile.number", { number: String(Math.max(0, Math.trunc(number))).padStart(3, "0") })}</div>
            <div className={`${styles.line} ${styles.name}`} title={name}>
              {name}
            </div>
            <div className={styles.typeLine}>
              <div className={styles.types}>
                {/* The game stores a single type twice. */}
                {[...new Set(types)].map((type) => (
                  <TypeBadge key={type} type={type} size="s" />
                ))}
              </div>
            </div>
            <Field label={t("dsPokedexProfile.category")} value={category} />
            <Field label={t("dsPokedexProfile.height")} value={height} />
            <Field label={t("dsPokedexProfile.weight")} value={weight} />
          </div>
          <div className={styles.textbox}>{text || t("dsPokedexProfile.textPending")}</div>
        </div>
      }
    />
  );
}
