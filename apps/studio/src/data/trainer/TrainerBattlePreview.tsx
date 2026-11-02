import type { BattleSpriteRequest } from "../../api";
import { DSBattlePreview, Segmented } from "../../ui/primitives";
import { battleSprite, battleSpriteKey, trainerSprite, trainerSpriteKey, useIcon } from "../icons";
import styles from "./Trainer.module.css";

/** The opponent's side of the battle preview: the trainer before the battle or the first Pokémon of its team. */
export type Foe = "trainer" | "pokemon";

/** The first Pokémon of a team as the battle shows it: its name, level and the battle picture to ask for. */
export interface FirstMon {
  name: string;
  level: number;
  sprite: BattleSpriteRequest;
}

/**
 * The DS battle screen with the opponent's side and the switch „Gegnerseite“ below it: the trainer's class picture (the box says `title`,
 * no level) or the first Pokémon of the team (front picture and level). One's own side stays empty: there is no player data to show.
 * Without a first Pokémon only the trainer is offered, whatever `foe` says. It reads no data context (the trainer view and the trainer
 * wizard share it); the pictures come from the icon cache.
 */
export function TrainerBattlePreview({ cls, title, first, foe, onFoe }: { cls: number; title: string; first: FirstMon | null; foe: Foe; onFoe: (foe: Foe) => void }) {
  const shown: Foe = first ? foe : "trainer";
  const monUrl = useIcon(first && shown === "pokemon" ? battleSpriteKey(first.sprite) : null, () => battleSprite(first!.sprite), true);
  const classUrl = useIcon(trainerSpriteKey(cls), () => trainerSprite(cls), true);
  const options = first
    ? [
        { value: "trainer" as const, label: "Trainer" },
        { value: "pokemon" as const, label: "Erstes Pokémon" },
      ]
    : [{ value: "trainer" as const, label: "Trainer" }];
  const side = shown === "pokemon" && first ? { name: first.name, level: first.level, spriteUrl: monUrl } : { name: title, level: 0, spriteUrl: classUrl };
  return (
    <>
      <DSBattlePreview foe={side} own={null} />
      <div className={styles.previewControls} data-foe={shown}>
        <Segmented options={options} value={shown} onChange={onFoe} ariaLabel="Gegnerseite" size="s" />
      </div>
    </>
  );
}
