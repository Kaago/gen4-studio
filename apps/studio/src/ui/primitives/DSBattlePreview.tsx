import type { Page } from "../../workbench/text/textModel";
import { DSScreen } from "./DSScreen";
import { Sprite } from "./Sprite";
import styles from "./DSBattlePreview.module.css";

export type BattleSide = {
  name: string;
  /** At most 0: the status box shows only the name, without „Lv.“ and HP bar (a trainer before the battle). */
  level: number;
  /** The 80 × 80 picture as a blob URL; `null` shows the placeholder. */
  spriteUrl: string | null;
  /** Share of HP left, 0..1; 1 when missing. */
  hp?: number;
};

export type DSBattlePreviewProps = {
  /** The opponent: picture top right, status box top left. */
  foe: BattleSide;
  /** One's own Pokémon: picture bottom left, status box bottom right; `null` leaves the ground empty, without box. */
  own: BattleSide | null;
  /** Whole-number magnification as in `DSScreen`; measured from the container when missing. */
  scale?: number;
  /** Only decoration, as in `DSScreen`: hidden from assistive technology. */
  decorative?: boolean;
};

type Which = "foe" | "own";
type HpColour = "gruen" | "gelb" | "rot";

const NO_PAGES: Page[] = [];

/** The game's thresholds: above half green, above a fifth yellow, else red. */
function hpColour(hp: number): HpColour {
  if (hp > 0.5) return "gruen";
  if (hp > 0.2) return "gelb";
  return "rot";
}

const hasLevel = (side: BattleSide) => side.level > 0;
const describe = (side: BattleSide) => (hasLevel(side) ? `${side.name} Lv. ${side.level}` : side.name);

function StatusBox({ side, which }: { side: BattleSide; which: Which }) {
  const hp = side.hp === undefined || !Number.isFinite(side.hp) ? 1 : Math.min(1, Math.max(0, side.hp));
  const percent = Math.round(hp * 100);
  const colour = hpColour(hp);
  return (
    <div className={`${styles.status} ${which === "foe" ? styles.foeStatus : styles.ownStatus}`} data-ds-status={which}>
      <div className={styles.statusLine}>{side.name}</div>
      {hasLevel(side) && (
        <div className={styles.statusLine}>
          <span>{`Lv. ${side.level}`}</span>
          <span
            role="meter"
            aria-label={`KP ${side.name}`}
            aria-valuenow={percent}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuetext={`${percent} Prozent`}
            className={styles.track}
            data-hp={colour}
          >
            <span className={`${styles.fill} ${styles[colour]}`} style={{ "--hp": hp } /* dyn */} />
          </span>
        </div>
      )}
    </div>
  );
}

/**
 * A battle on the DS screen with fixed places: the opponent top right on its ground with its status box top left, one's own Pokémon bottom
 * left with its box bottom right. Drawn into the background of an empty `DSScreen` (same frame, scale and game font).
 */
export function DSBattlePreview({ foe, own, scale, decorative = false }: DSBattlePreviewProps) {
  const label = `Kampfvorschau: ${describe(foe)}${own ? ` gegen ${describe(own)}` : ""}`;
  return (
    <DSScreen
      variant="empty"
      pages={NO_PAGES}
      page={0}
      scale={scale}
      decorative={decorative}
      label={label}
      background={
        <div className={styles.battle} data-ds-battle="">
          <div className={`${styles.ground} ${styles.foeGround}`} data-ds-ground="foe" />
          <div className={`${styles.ground} ${styles.ownGround}`} data-ds-ground="own" />
          {/* The names stand in the status boxes. */}
          <div className={`${styles.picture} ${styles.foePicture}`}>
            <Sprite url={foe.spriteUrl} label={foe.name} size={80} area="pokemon" decorative />
          </div>
          {own && (
            <div className={`${styles.picture} ${styles.ownPicture}`}>
              <Sprite url={own.spriteUrl} label={own.name} size={80} area="pokemon" decorative />
            </div>
          )}
          <StatusBox side={foe} which="foe" />
          {own && <StatusBox side={own} which="own" />}
        </div>
      }
    />
  );
}
