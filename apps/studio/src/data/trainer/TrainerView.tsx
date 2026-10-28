import { useEffect, useMemo, useState } from "react";
import { getEvents } from "../../api";
import type { HeaderSummary, RefDto } from "../../api";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { EntityIdLine } from "../../textproject/EntityIdLine";
import { renameEntry } from "../../textproject/renameEntry";
import { useTextProject } from "../../textproject/TextProjectContext";
import { useEntityIds } from "../../textproject/useEntityIds";
import { Badge, Button, Notice, PokemonCard, Reference } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { monIcon, monIconKey, useIcon } from "../icons";
import { itemName } from "../pokedex/pokedexModel";
import { useDataArea } from "../useDataArea";
import { PLACEHOLDER_TEXT, docTitle, isDouble, speciesName } from "./trainerModel";
import type { TrainerDoc } from "./trainerModel";
import { useTrainer } from "./useTrainer";
import styles from "./Trainer.module.css";

/** A party Pokémon as a card that opens its species in the Pokédex. */
function TeamCard({ doc, slot }: { doc: TrainerDoc; slot: number }) {
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
      ariaLabel={`Platz ${slot + 1}: ${name}, Level ${mon.level}, im Pokédex öffnen`}
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
  if (!header) return { name: `Eventdatei ${id4(file)}`, header: null };
  return { name: header.location_name || header.internal_name || `Header ${id4(header.id)}`, header: header.id };
}

/** „Steht in der Welt“: the object events that start the battle, each with its place and tile, opening it in the world. */
function InWorld({ doc, index }: { doc: TrainerDoc; index: number }) {
  const { headers, goTo, placeTrainer } = useDataArea("pokemon");
  const tiles = useEventTiles(doc.events);
  return (
    <section className={styles.section} aria-label="Steht in der Welt">
      <h3 className={styles.sectionTitle}>Steht in der Welt</h3>
      {doc.events.length === 0 ? (
        <>
          <p className={styles.note}>Noch nicht platziert: ohne Figur in der Welt kämpft niemand gegen diesen Trainer.</p>
          <span>
            <Button kind="secondary" size="s" icon="figure" disabled={placeTrainer === null} title={placeTrainer === null ? "Platzieren kommt mit dem Trainer-Feld der Welt" : undefined} onClick={() => placeTrainer?.(index)}>
              In der Welt platzieren
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
            const label = tile ? `${place.name}, Kachel ${tile.x}, ${tile.z}` : `${place.name}, Objekt ${object}`;
            return (
              <Reference
                key={`${eventFile}:${object}`}
                area="welt"
                label={label}
                tech={`Eventdatei ${id4(eventFile)}`}
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
 * The view (Schau) of a trainer: class and name, a double battle, the team as cards (a card opens its species), where the trainer stands
 * in the world or the way to place it, a warning for an empty team (the battle crashes). The placeholder of the sight script says what it
 * is instead.
 */
export function TrainerView() {
  const { row, labelOf, names, selected } = useDataArea("pokemon");
  const { doc, error } = useTrainer(selected);
  const reset = useReset();
  // In a text project the head names the trainer's id and offers to rename it.
  const text = useTextProject();
  const ids = useEntityIds("Trainer", text.active);
  if (row === null || selected === null) return null;
  const entityId = ids?.[selected] ?? null;
  const title = doc && !doc.placeholder && doc.locked === null ? docTitle(doc, names.classes, selected) : labelOf(row);
  return (
    <section className={styles.view} aria-label={`Übersicht ${title}`} data-trainer={selected}>
      <div className={styles.viewBody}>
        <header className={styles.head}>
          <h2 className={styles.name}>{title}</h2>
          <span className={styles.tech}>{row.tech}</span>
          {entityId !== null && <EntityIdLine id={entityId} />}
          {doc && isDouble(doc.battle_type) && <Badge kind="info">Doppelkampf</Badge>}
          {/* Properties and team; the name and the texts stay (they are text archives). The placeholder is refused with its reason. */}
          <HeadActions items={[resetEntry({ kind: "trainer", index: selected }, reset.status, reset.open), ...(entityId !== null ? [renameEntry(entityId, text.renameId)] : [])]} />
        </header>
        {error !== null && <Notice kind="error">{`Dieser Trainer ist nicht lesbar: ${error}`}</Notice>}
        {doc?.placeholder && <Notice kind="info">{`${PLACEHOLDER_TEXT}: hält die Nummer frei, die das Kampf-Script für „gesehen“ nutzt. Er wird nie bearbeitet.`}</Notice>}
        {doc && !doc.placeholder && doc.locked !== null && <Notice kind="warning">{`${lockReason(doc.locked)}. Dieser Trainer ist gesperrt.`}</Notice>}
        {doc && !doc.placeholder && doc.locked === null && doc.party.length === 0 && (
          <Notice kind="warning">Ohne Pokémon stürzt der Kampf ab. Füge in der Hand unter Team ein Pokémon hinzu.</Notice>
        )}
        {doc && doc.party.length > 0 && (
          <section className={styles.section} aria-label="Team">
            <h3 className={styles.sectionTitle}>
              Team <span className={styles.count}>{`${doc.party.length} von 6`}</span>
            </h3>
            <div className={styles.team}>
              {doc.party.map((_, slot) => (
                <TeamCard key={slot} doc={doc} slot={slot} />
              ))}
            </div>
          </section>
        )}
        {doc && !doc.placeholder && <InWorld doc={doc} index={selected} />}
        {doc === null && error === null && <p className={styles.note}>Lädt …</p>}
      </div>
    </section>
  );
}
