import { useState } from "react";
import type { NameTablesDto } from "../../api";
import { monIcon, monIconKey, useIcon } from "../../data/icons";
import { ClassSprite } from "../../data/trainer/ClassSprite";
import { TrainerBattlePreview } from "../../data/trainer/TrainerBattlePreview";
import type { Foe, FirstMon } from "../../data/trainer/TrainerBattlePreview";
import { className, speciesName } from "../../data/trainer/trainerModel";
import { PokemonCard } from "../../ui/primitives";
import type { RowDataAttrs } from "../../ui/primitives";
import styles from "./TrainerPreview.module.css";

/** One Pokémon of the team as the trainer wizard asks for it; the species is null until one is picked. */
export interface TeamRow {
  species: number | null;
  level: number;
}

/** What a trainer without a name is called in the wizard (its summary says the same). */
export const NO_NAME = "Ohne Namen";

/**
 * A place of the wizard's team as a card with the species' menu icon (from the icon cache, no data context); a place without a species is
 * the dashed free card. The rows of step „Team“ and the cards of the preview are the same card; `labelPrefix` tells the preview's apart
 * („Vorschau, Platz 1: …“).
 */
export function TeamRowCard({
  row,
  slot,
  names,
  onClick,
  dataAttrs,
  labelPrefix = "",
}: {
  row: TeamRow;
  slot: number;
  names: NameTablesDto | null;
  onClick: () => void;
  dataAttrs?: RowDataAttrs;
  labelPrefix?: string;
}) {
  const species = row.species;
  const iconUrl = useIcon(species === null ? null : monIconKey(species), () => monIcon(species ?? 0), true);
  const name = species === null ? undefined : speciesName(names?.species ?? [], species);
  return (
    <PokemonCard
      state={species === null ? "empty" : "idle"}
      species={species ?? undefined}
      name={name}
      level={species === null ? undefined : row.level}
      iconUrl={iconUrl}
      slot={slot + 1}
      ariaLabel={`${labelPrefix}${name === undefined ? `Platz ${slot + 1}: leer` : `Platz ${slot + 1}: ${name}, Level ${row.level}`}`}
      onClick={onClick}
      dataAttrs={dataAttrs}
    />
  );
}

export interface TrainerPreviewProps {
  cls: number;
  name: string;
  party: TeamRow[];
  /** Null while the name tables load: classes and species go by number. */
  names: NameTablesDto | null;
  classes: string[];
  /** A card of the preview is clicked: the wizard lets the species of that place be picked. */
  onSlot: (slot: number) => void;
}

/**
 * The live preview beside every step of „Neuer Trainer hier“: the picture of the class with name and class, the battle as the game opens
 * it (`TrainerBattlePreview`: the trainer, or the first place of the team once it has a species; the choice stays across the steps) and the
 * team as compact cards. It follows the form as it is typed; it reads no data context (the wizard runs outside the data areas).
 */
export function TrainerPreview({ cls, name, party, names, classes, onSlot }: TrainerPreviewProps) {
  const [foe, setFoe] = useState<Foe>("trainer");
  const clsName = className(classes, cls);
  const own = name.trim();
  const title = own === "" ? clsName : `${clsName} ${own}`;
  const mon = party[0];
  const first: FirstMon | null =
    mon && mon.species !== null ? { name: speciesName(names?.species ?? [], mon.species), level: mon.level, sprite: { species: mon.species, side: "front" } } : null;
  return (
    <section className={styles.preview} aria-label="Vorschau des Trainers" data-trainer-preview="">
      <header className={styles.head}>
        <ClassSprite cls={cls} name={title} size={80} />
        <div className={styles.headText}>
          <span className={styles.name}>{own === "" ? NO_NAME : own}</span>
          <span className={styles.cls}>{clsName}</span>
        </div>
      </header>
      <h3 className={styles.heading}>Kampfvorschau</h3>
      <TrainerBattlePreview cls={cls} title={title} first={first} foe={foe} onFoe={setFoe} />
      <h3 className={styles.heading}>Team</h3>
      <div className={styles.team}>
        {party.map((row, i) => (
          <TeamRowCard key={i} row={row} slot={i} names={names} onClick={() => onSlot(i)} dataAttrs={{ "data-preview-slot": i }} labelPrefix="Vorschau, " />
        ))}
      </div>
    </section>
  );
}
