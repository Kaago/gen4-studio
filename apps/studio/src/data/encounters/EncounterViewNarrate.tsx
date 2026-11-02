import { useMemo } from "react";
import type { NameTablesDto } from "../../api";
import { Badge, EmptyState, InspectorRow, Notice, NumberField, Segmented, Sprite } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { monIcon, monIconKey, useIcon } from "../icons";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { PickField } from "../pokedex/pickers";
import { isEvolutionTarget } from "../pokedex/pokedexModel";
import { speciesName } from "../trainer/trainerModel";
import { useDataArea } from "../useDataArea";
import type { DocEdit } from "../useDataDoc";
import { TIME_COLUMNS, TIME_HOURS, entryLabel, listWord, rateLabel, shareText, slotLabel, slotText, swapLabel } from "./encounterModel";
import type { EncounterDoc, TimeKey } from "./encounterModel";
import { FileWarnings, Users } from "./EncounterView";
import { RARITIES, rarityOf, rarityWord, slotRarity, swapTarget, timeSlot } from "./rarity";
import type { Rarity, TimeSlot } from "./rarity";
import { useEncounter } from "./useEncounter";
import styles from "./EncounterViewNarrate.module.css";

/** What the rows write with: the file as shown, its title for the labels, and one edit as one undo step. */
interface Editing {
  doc: EncounterDoc;
  title: string;
  names: NameTablesDto;
  family: string;
  locked: boolean;
  write(make: (now: EncounterDoc) => DocEdit | null): void;
}

/** The file of the selected row with its queue of edits, its title and how one edit is written; null fields while there is none. */
function useEditing() {
  const { row, labelOf, names, selected, project } = useDataArea("pokemon");
  const { doc, error, editError, update } = useEncounter(selected);
  const index = selected ?? 0;
  const family = project?.family ?? "Pt";
  const title = row === null ? `Begegnungen ${id4(index)}` : labelOf(row);
  const editing = useMemo<Editing | null>(() => {
    if (doc === null) return null;
    const locked = doc.locked !== null;
    return {
      doc,
      title,
      names,
      family,
      locked,
      write: (make) => {
        if (locked) return;
        void update(index, (now) => (now.locked !== null ? null : make(now)));
      },
    };
  }, [doc, title, names, family, index, update]);
  return { row, selected, index, title, editing, error, editError };
}

/** A species by name; 0 is „Keine“. */
const speciesText = (names: NameTablesDto, id: number) => (id === 0 ? "Keine" : speciesName(names.species, id));

/** The species written at `at` in `doc` (a grass slot's column or a list entry); null when the file has no such place. */
function speciesAt(doc: EncounterDoc, at: TimeSlot): number | null {
  if (at.method !== "grass") return doc.replacements.find((l) => l.list === at.method)?.slots.find((s) => s.index === at.slot)?.species ?? null;
  const slot = doc.methods.find((m) => m.method === "grass")?.slots.find((s) => s.index === at.slot);
  if (slot === undefined) return null;
  return at.field === "species" ? slot.species : (slot[at.field] ?? 0);
}

/** The level of grass slot `slot` in `doc`; null without it. */
const levelAt = (doc: EncounterDoc, slot: number): number | null => doc.methods.find((m) => m.method === "grass")?.slots.find((s) => s.index === slot)?.min ?? null;

/** Why a row's class cannot be switched: Platinum takes its place by day or by night from a replacement list, which a swap cannot move. */
const listTitle = (at: TimeSlot, word: string): string =>
  `Kommt ${word} aus „${listWord(at.method)}“ Eintrag ${at.slot + 1} und ersetzt Gras ${slotText(at.grass)}. Ein Ersatzeintrag lässt sich nicht auf einen anderen Platz tauschen.`;

/**
 * One species waiting at one time of day: picture, name, its share as a badge („24 % · häufig“), and what changes it there: the species
 * (the place it has at this time: a grass slot's column, or Platinum's list entry by day and by night), the level of its grass slot, and
 * its class (häufig · gewöhnlich · selten), which trades its grass slot with the first fitting slot of the chosen class (`swapTarget`) in one
 * undo step.
 */
function WaitingRow({ e, when, label, word, species, share }: { e: Editing; when: TimeKey; label: string; word: string; species: number; share: number }) {
  const name = speciesName(e.names.species, species);
  const icon = useIcon(monIconKey(species), () => monIcon(species), true);
  const at = timeSlot(e.doc, e.family, when, species);
  const level = at === null ? null : levelAt(e.doc, at.grass);
  const fromList = at !== null && at.method !== "grass";
  const rarity: Rarity = (at === null ? null : slotRarity(at.grass)) ?? rarityOf(share);
  const fixed = e.locked || at === null || fromList;

  // Each edit finds the species' place in the file as it is when the edit runs, not as it was shown.
  const pick = (value: number) =>
    e.write((now) => {
      const place = timeSlot(now, e.family, when, species);
      const old = place === null ? null : speciesAt(now, place);
      if (place === null || old === null || old === value) return null;
      const words = `${speciesText(e.names, old)} → ${speciesText(e.names, value)}`;
      if (place.method !== "grass") return { patch: { Encounter: { method: place.method, slot: place.slot, field: "species", value } }, label: entryLabel(e.title, place.method, place.slot, words) };
      // HeartGold names the column („Gras Platz 1 nachts Hoothoot → Taubsi“).
      const column = e.family === "Hgss" ? `${TIME_COLUMNS.find((c) => c.field === place.field)?.word ?? ""} ` : "";
      return { patch: { Encounter: { method: "grass", slot: place.slot, field: place.field, value } }, label: slotLabel(e.title, "grass", place.slot, `${column}${words}`) };
    });
  const setLevel = (value: number) =>
    e.write((now) => {
      const place = timeSlot(now, e.family, when, species);
      const old = place === null ? null : levelAt(now, place.grass);
      return place === null || old === null || old === value ? null : { patch: { Encounter: { method: "grass", slot: place.grass, field: "level", value } }, label: slotLabel(e.title, "grass", place.grass, `Level ${old} → ${value}`) };
    });
  // Named as a result: the species' place at this time is of class `to`. Already there (or no longer waiting here) writes nothing.
  const setRarity = (to: Rarity) =>
    e.write((now) => {
      const here = timeSlot(now, e.family, when, species);
      if (here === null || here.method !== "grass") return null;
      const target = swapTarget(now, e.family, when, here.grass, to);
      return target === null ? null : { patch: { EncounterSwap: { method: "grass", a: here.grass, b: target } }, label: swapLabel(e.title, "grass", here.grass, target) };
    });

  return (
    <li className={styles.row} data-waiting={when} data-species={species}>
      <div className={styles.who}>
        <Sprite url={icon} label={name} size={32} area="pokemon" decorative />
        <span className={styles.name}>{name}</span>
        <span className={styles.share} data-share="">
          <Badge kind="neutral">{`${shareText(share)} · ${rarityWord(rarityOf(share))}`}</Badge>
        </span>
      </div>
      <div className={styles.controls}>
        <PickField label={`Pokémon ${word}`} text={name} choice="species" only={isEvolutionTarget} disabled={e.locked || at === null} onPick={pick} />
        <span className={styles.field}>
          <span className={styles.fieldWord} aria-hidden="true">
            Level
          </span>
          <NumberField label={`Level ${name} ${label}`} value={level ?? 0} bits={8} disabled={e.locked || level === null} onCommit={setLevel} />
        </span>
        <span className={styles.rarity} data-rarity={rarity} title={at !== null && fromList ? listTitle(at, word) : undefined}>
          <Segmented ariaLabel={`Wie häufig ${name} ${label}`} size="s" options={RARITIES.map((r) => ({ ...r, disabled: fixed }))} value={rarity} onChange={setRarity} />
        </span>
      </div>
    </li>
  );
}

/** One time of day: who waits in the grass then, largest share first (the kernel's sums). */
function TimeColumn({ e, when, label, word }: { e: Editing; when: TimeKey; label: string; word: string }) {
  const waiting = e.doc.effective.find((t) => t.when === when)?.species ?? [];
  return (
    <section className={styles.column} aria-label={label} data-time={when}>
      <h3 className={styles.columnTitle}>
        {label} <span className={styles.hours}>{TIME_HOURS[when]}</span>
      </h3>
      {waiting.length === 0 ? (
        <p className={styles.note}>Niemand.</p>
      ) : (
        <ul className={styles.rows}>
          {waiting.map((w) => (
            <WaitingRow key={w.species} e={e} when={when} label={label} word={word} species={w.species} share={w.share} />
          ))}
        </ul>
      )}
    </section>
  );
}

/**
 * The view (Schau) of an encounter file in Erzählen: „Wer hier wartet“ in three columns (Morgens · Tags · Nachts), each species with its
 * picker, level and class. Surfen and Angeln stay in Bauen. The rules the file breaks follow below.
 */
export function EncounterViewNarrate() {
  const { row, selected, title, editing, error, editError } = useEditing();
  const reset = useReset();
  if (row === null || selected === null) return null;
  const doc = editing?.doc ?? null;
  const grass = doc?.methods.find((m) => m.method === "grass");
  return (
    <section className={styles.view} aria-label={`Übersicht ${title}`} data-encounter={selected} data-encounter-narrate="">
      <header className={styles.head}>
        <h2 className={styles.title}>{title}</h2>
        <HeadActions items={[resetEntry({ kind: "encounter", index: selected }, reset.status, reset.open)]} />
      </header>
      {error !== null && <Notice kind="error">{`Diese Begegnungsdatei ist nicht lesbar: ${error}`}</Notice>}
      {editError !== null && <Notice kind="error">{editError}</Notice>}
      {doc !== null && doc.locked !== null && <Notice kind="warning">{`${lockReason(doc.locked)}. Diese Datei ist gesperrt.`}</Notice>}
      {editing !== null && editing.doc.locked === null && (
        <section className={styles.waiting} aria-label="Wer hier wartet">
          <h3 className={styles.sectionTitle}>
            Wer hier wartet <span className={styles.hours}>im Gras</span>
          </h3>
          {grass !== undefined && grass.rate === 0 && <p className={styles.note}>Wie oft 0: im Gras gibt es hier keine Begegnungen.</p>}
          <div className={styles.columns}>
            {TIME_COLUMNS.map((c) => (
              <TimeColumn key={c.when} e={editing} when={c.when} label={c.label} word={c.word} />
            ))}
          </div>
        </section>
      )}
      {doc !== null && <FileWarnings doc={doc} />}
      {doc === null && error === null && <p className={styles.note}>Lädt …</p>}
    </section>
  );
}

/** The hand of an encounter file in Erzählen: how often the grass gives an encounter (all three times share it), and who uses the file. */
export function EncounterHandNarrate() {
  const { row, selected, title, editing, error, editError } = useEditing();
  if (row === null || selected === null) return null;
  if (editing === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what="Diese Begegnungsdatei ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}</div>;
  }
  const grass = editing.doc.methods.find((m) => m.method === "grass");
  const setRate = (value: number) =>
    editing.write((now) => {
      const old = now.methods.find((m) => m.method === "grass")?.rate;
      return old === undefined || old === value ? null : { patch: { Encounter: { method: "grass", slot: null, field: "rate", value } }, label: rateLabel(title, "grass", old, value) };
    });
  return (
    <div className={styles.hand} data-encounter={selected} data-encounter-narrate="">
      {editing.locked && <Notice kind="warning">{`${lockReason(editing.doc.locked ?? "")}. Die Felder dieser Datei sind gesperrt.`}</Notice>}
      {editError !== null && <Notice kind="error">{editError}</Notice>}
      {grass !== undefined && (
        <InspectorRow label="Wie oft">
          <NumberField
            label="Wie oft (0 = nie)"
            value={grass.rate}
            min={0}
            max={255}
            disabled={editing.locked}
            hint={editing.family === "Hgss" ? "Im Gras, morgens, tags und nachts gleich." : "Im Gras, morgens, tags und nachts gleich. Über 100 wirkt wie 100."}
            onCommit={setRate}
          />
        </InspectorRow>
      )}
      <Users doc={editing.doc} />
    </div>
  );
}
