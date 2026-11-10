import { useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { getEvents } from "../../api";
import type { HeaderSummary, RefDto } from "../../api";
import { useDelete, useEntityState } from "../../delete/DeleteContext";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { EntityIdLine } from "../../textproject/EntityIdLine";
import { renameEntry } from "../../textproject/renameEntry";
import { useTextProject } from "../../textproject/TextProjectContext";
import { useEntityIds } from "../../textproject/useEntityIds";
import { Badge, Button, Notice, PokemonCard, Reference } from "../../ui/primitives";
import { id4, termLabel } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { monIcon, monIconKey, useIcon } from "../icons";
import { itemName } from "../pokedex/pokedexModel";
import { useDataArea } from "../useDataArea";
import { docTitle, isDouble, placeholderText, speciesName } from "./trainerModel";
import type { TrainerDoc } from "./trainerModel";
import { ClassSprite } from "./ClassSprite";
import { TrainerBattlePreview } from "./TrainerBattlePreview";
import type { Foe, FirstMon } from "./TrainerBattlePreview";
import { useTrainer } from "./useTrainer";
import styles from "./Trainer.module.css";

/** A party Pokémon as a card that opens its species in the Pokédex. */
function TeamCard({ doc, slot }: { doc: TrainerDoc; slot: number }) {
  const { t } = useTranslation("data");
  const { names, goTo } = useDataArea("pokemon");
  const mon = doc.party[slot]!;
  const name = speciesName(names.species, mon.species);
  const icon = useIcon(monIconKey(mon.species), () => monIcon(mon.species), true);
  return (
    <PokemonCard
      state="idle"
      species={mon.species}
      name={name}
      level={mon.level}
      item={mon.item ? itemName(names.items, mon.item) : undefined}
      iconUrl={icon}
      slot={slot + 1}
      ariaLabel={t("trainerView.teamCard", { slot: slot + 1, name, level: mon.level })}
      onClick={() => goTo({ kind: "species", index: mon.species })}
      dataAttrs={{ "data-slot": slot }}
    />
  );
}

/** Where an object event stands: its tile, read from its event file (null while it is read or when it cannot be). */
function useEventTiles(events: readonly RefDto[]): ReadonlyMap<string, { x: number; z: number }> | null {
  const files = useMemo(() => [...new Set(events.map((e) => e.source.index))], [events]);
  const key = files.join(",");
  const [tiles, setTiles] = useState<{ key: string; map: Map<string, { x: number; z: number }> } | null>(null);
  useEffect(() => {
    let current = true;
    Promise.all(files.map((f) => getEvents(f).catch(() => null))).then(
      (all) => {
        if (!current) return;
        const map = new Map<string, { x: number; z: number }>();
        for (const dto of all) for (const o of dto?.objects ?? []) map.set(`${dto!.event_file_id}:${o.index}`, { x: o.x, z: o.z });
        setTiles({ key, map });
      },
      () => current && setTiles({ key, map: new Map() }),
    );
    return () => {
      current = false;
    };
    // `key` stands for `files`.
  }, [key]);
  return tiles !== null && tiles.key === key ? tiles.map : null;
}

/** The place of an event file: the location of the first header that loads it, else the file. */
function placeOf(file: number, headers: readonly HeaderSummary[]): { name: string; header: number | null } {
  const header = headers.find((h) => h.event_file_id === file);
  if (!header) return { name: `${termLabel("eventFile")} ${id4(file)}`, header: null };
  return { name: header.location_name || header.internal_name || `${termLabel("header")} ${id4(header.id)}`, header: header.id };
}

/**
 * „Steht in der Welt“: the object events that start the battle, each with its place and tile, opening it in the world; from Bauen on with
 * its event file.
 */
function InWorld({ doc, index, narrating }: { doc: TrainerDoc; index: number; narrating: boolean }) {
  const { t } = useTranslation("data");
  const { headers, goTo, placeTrainer } = useDataArea("pokemon");
  const tiles = useEventTiles(doc.events);
  return (
    <section className={styles.section} aria-label={t("trainerView.inWorld")}>
      <h3 className={styles.sectionTitle}>{t("trainerView.inWorld")}</h3>
      {doc.events.length === 0 ? (
        <>
          <p className={styles.note}>{t("trainerView.notPlaced")}</p>
          <span>
            <Button kind="secondary" size="s" icon="figure" disabled={placeTrainer === null} title={placeTrainer === null ? t("trainerView.placeSoon") : undefined} onClick={() => placeTrainer?.(index)}>
              {t("trainerView.placeInWorld")}
            </Button>
          </span>
        </>
      ) : (
        <div className={styles.chips}>
          {doc.events.map((r) => {
            if (r.nav.kind !== "event") return null;
            const { eventFile, index: object } = r.nav;
            const place = placeOf(eventFile, headers);
            const tile = tiles?.get(`${eventFile}:${object}`);
            const label = tile ? t("trainerView.chipTile", { place: place.name, x: tile.x, z: tile.z }) : t("trainerView.chipObject", { place: place.name, object });
            return (
              <Reference
                key={`${eventFile}:${object}`}
                area="welt"
                label={label}
                tech={narrating ? undefined : `${termLabel("eventFile")} ${id4(eventFile)}`}
                onOpen={() => goTo({ kind: "event", eventFile, eventKind: "object", index: object, header: place.header })}
                dataAttrs={{ "data-event": object }}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}

/**
 * "Kampfvorschau": the shared battle preview of the trainer (`TrainerBattlePreview`) with the first Pokémon of the party. Only an explicit ♀
 * asks for the female picture (HeartGold's gender field; Platinum has none, and the low nibble of byte 1 is an IV byte there, so `genderOf`
 * is not used). The backend falls back to the other gender when a species has no picture of that one.
 */
function BattlePreview({ doc, title, foe, onFoe }: { doc: TrainerDoc; title: string; foe: Foe; onFoe: Dispatch<SetStateAction<Foe>> }) {
  const { t } = useTranslation("data");
  const { names } = useDataArea("pokemon");
  const mon = doc.party[0];
  const first: FirstMon | null = mon
    ? { name: speciesName(names.species, mon.species), level: mon.level, sprite: { species: mon.species, ...(mon.form > 0 ? { form: mon.form } : {}), side: "front", ...(mon.gender === 2 ? { female: true } : {}) } }
    : null;
  return (
    <section className={styles.section} aria-label={t("trainerView.battlePreview")} data-ds-battle-section="">
      <h3 className={styles.sectionTitle}>{t("trainerView.battlePreview")}</h3>
      <TrainerBattlePreview cls={doc.class} title={title} first={first} foe={foe} onFoe={onFoe} />
    </section>
  );
}

/**
 * The view (Schau) of a trainer: class and name, a double battle, the team as cards (a card opens its species), where the trainer stands
 * in the world or the way to place it, a warning for an empty team (the battle crashes). The placeholder of the sight script says what it
 * is instead. `narrating` (Erzählen): without the trainer's number and the event files.
 */
export function TrainerView({ narrating = false }: { narrating?: boolean }) {
  const { t } = useTranslation("data");
  const { row, labelOf, names, selected } = useDataArea("pokemon");
  const { doc, error } = useTrainer(selected);
  // The opponent's side of the preview belongs to the view for the session, so the loading gap between two trainers does not reset it.
  const [foe, setFoe] = useState<Foe>("trainer");
  const reset = useReset();
  const remove = useDelete();
  const state = useEntityState(selected === null ? null : { kind: "trainer", index: selected }, row);
  // In a text project the head names the trainer's id and offers to rename it.
  const text = useTextProject();
  const ids = useEntityIds("Trainer", text.active);
  if (row === null || selected === null) return null;
  const entityId = ids?.[selected] ?? null;
  const usable = doc !== null && !doc.placeholder && doc.locked === null;
  const title = usable ? docTitle(doc, names.classes, selected) : labelOf(row);
  return (
    <section className={styles.view} aria-label={t("trainerView.overview", { title })} data-trainer={selected}>
      <div className={styles.viewBody}>
        <header className={styles.head}>
          <ClassSprite cls={usable ? doc.class : null} name={title} size={80} />
          <div className={styles.headText}>
            <h2 className={styles.name}>{title}</h2>
            {!narrating && <span className={styles.tech}>{row.tech}</span>}
            {entityId !== null && <EntityIdLine id={entityId} />}
            {doc && isDouble(doc.battle_type) && <Badge kind="info">{t("trainerView.doubleBattle")}</Badge>}
          </div>
          {/* Deleting leaves a tombstone. Resetting: properties and team; the name and the texts stay (they are text archives). The
              placeholder is refused with its reason. */}
          <HeadActions
            state={state}
            items={[
              { label: t("trainerView.delete"), onSelect: () => remove.open({ kind: "trainer", index: selected }) },
              resetEntry({ kind: "trainer", index: selected }, reset.status, reset.open),
              ...(entityId !== null ? [renameEntry(entityId, text.renameId)] : []),
            ]}
          />
        </header>
        {error !== null && <Notice kind="error">{t("trainerView.unreadable", { error })}</Notice>}
        {doc?.placeholder && <Notice kind="info">{t("trainerView.placeholderNote", { placeholder: placeholderText() })}</Notice>}
        {doc && !doc.placeholder && doc.locked !== null && <Notice kind="warning">{t("trainerView.locked", { reason: lockReason(doc.locked) })}</Notice>}
        {doc && !doc.placeholder && doc.locked === null && doc.party.length === 0 && (
          <Notice kind="warning">{t("trainerView.emptyParty")}</Notice>
        )}
        {doc && doc.party.length > 0 && (
          <section className={styles.section} aria-label={t("trainerView.party")}>
            <h3 className={styles.sectionTitle}>
              {t("trainerView.party")} <span className={styles.count}>{t("trainerView.partyOf", { n: doc.party.length })}</span>
            </h3>
            <div className={styles.team}>
              {doc.party.map((_, slot) => (
                <TeamCard key={slot} doc={doc} slot={slot} />
              ))}
            </div>
          </section>
        )}
        {usable && <BattlePreview doc={doc} title={title} foe={foe} onFoe={setFoe} />}
        {doc && !doc.placeholder && <InWorld doc={doc} index={selected} narrating={narrating} />}
        {doc === null && error === null && <p className={styles.note}>{t("loading")}</p>}
      </div>
    </section>
  );
}

/** The view of a trainer in Erzählen (`DataPart.ViewNarrate`): the same view without numbers. */
export function TrainerViewNarrate() {
  return <TrainerView narrating />;
}
