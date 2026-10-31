import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { dataGet, dataRaw, errorText } from "../../api";
import type { NameTablesDto, PartyMonDto, RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { Button, Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, PokemonCard, Segmented, SelectField, Switch, Tabs, TextField } from "../../ui/primitives";
import type { SelectOption, TabItem } from "../../ui/primitives";
import { lockReason } from "../dataModel";
import { monIcon, monIconKey, useIcon } from "../icons";
import { ChoicePicker, PickField, useChoices } from "../pokedex/pickers";
import { isEvolutionTarget, itemName, moveName } from "../pokedex/pokedexModel";
import { useDataArea } from "../useDataArea";
import type { DocEdit } from "../useDataDoc";
import {
  ABILITY_OPTIONS,
  AI_LEVELS,
  AI_OPTIONS,
  BATTLE_OPTIONS,
  GENDER_OPTIONS,
  PARTY_MAX,
  PLACEHOLDER_TEXT,
  TEXT_KINDS,
  abilityOf,
  aiLevel,
  aiText,
  battleText,
  bitLabel,
  classOptions,
  className,
  defaultMoves,
  docTitle,
  formCount,
  genderOf,
  isDouble,
  isMarkProblem,
  nameCount,
  nameError,
  newMon,
  optionText,
  problemText,
  slotLabel,
  speciesName,
  strengthField,
  strengthOf,
  textKind,
  withBit,
  withDouble,
  withMon,
  withStrength,
} from "./trainerModel";
import type { TrainerDoc } from "./trainerModel";
import { ClassSprite } from "./ClassSprite";
import { useTrainer } from "./useTrainer";
import styles from "./Trainer.module.css";

type TrainerTab = "team" | "trainer" | "raw";

/** What every part of the editor works with: the trainer as shown, the names, and how to write one edit. */
interface Editing {
  index: number;
  doc: TrainerDoc;
  names: NameTablesDto;
  family: string;
  /** The placeholder or a trainer of the wrong length: every field is disabled and nothing is written. */
  locked: boolean;
  showBase: boolean;
  /**
   * Writes one edit as one undo step, built from the trainer as the backend holds it when the edit runs (`useDataDoc`), with its title
   * ("Wanderer Jens") for the label; `make` returns null for nothing to write. A locked trainer writes nothing.
   */
  write(make: (now: TrainerDoc, title: string) => DocEdit | null): void;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
const moveText = (names: NameTablesDto, id: number) => (id === 0 ? "Keine" : moveName(names.moves, id));

/** A column of the hand: a bold title with its grey note. */
function Column({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className={styles.columnBox} aria-label={title}>
      <h3 className={styles.columnTitle}>
        {title}
        {note && <span className={styles.columnNote}>{note}</span>}
      </h3>
      {children}
    </section>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- team

/** One card of the hand's team: a filled place selects its details, a free one adds a Pokémon. */
function SlotCard({ e, slot, chosen, onChoose, onAdd }: { e: Editing; slot: number; chosen: boolean; onChoose: () => void; onAdd: () => void }) {
  const mon = e.doc.party[slot];
  const species = mon?.species ?? 0;
  const icon = useIcon(mon ? monIconKey(species) : null, () => monIcon(species), true);
  if (!mon) {
    return <PokemonCard state="empty" slot={slot + 1} ariaLabel={`Platz ${slot + 1} leer, Pokémon hinzufügen`} onClick={onAdd} disabled={e.locked} dataAttrs={{ "data-slot": slot }} />;
  }
  const name = speciesName(e.names.species, mon.species);
  return (
    <PokemonCard
      state={chosen ? "selected" : "idle"}
      species={mon.species}
      name={name}
      level={mon.level}
      item={mon.item ? itemName(e.names.items, mon.item) : undefined}
      iconUrl={icon}
      slot={slot + 1}
      ariaLabel={`Platz ${slot + 1}: ${name}, Level ${mon.level}`}
      onClick={onChoose}
      disabled={e.locked}
      dataAttrs={{ "data-slot": slot }}
    />
  );
}

/** A field of the place details, with the place in the field's name for the picker buttons. */
function Row({ label, children }: { label: string; children: ReactNode }) {
  return <InspectorRow label={label}>{children}</InspectorRow>;
}

/**
 * The details of one place: species (a picker with type badges), level, strength, HeartGold's gender and ability, the form for species with
 * forms, the item with items on, four moves with own moves on, and removing the Pokémon. Each change writes the whole party as one step.
 */
function SlotDetails({ e, slot, onRemoved }: { e: Editing; slot: number; onRemoved: () => void }) {
  const mon = e.doc.party[slot]!;
  const place = `Platz ${slot + 1}`;
  const hgss = e.family === "Hgss";
  const strength = strengthField(e.family);
  const forms = formCount(mon.species);
  const off = e.locked;
  /** Writes the Pokémon at this place changed by `change`, labelled "<title>: Platz 1 <rest>". */
  const setMon = (change: (m: PartyMonDto) => PartyMonDto, rest: (old: PartyMonDto, next: PartyMonDto) => string) =>
    e.write((now, title) => {
      const old = now.party[slot];
      if (!old) return null;
      const next = change(old);
      return same(old, next) ? null : { patch: { TrainerParty: { party: withMon(now.party, slot, next) } }, label: slotLabel(title, slot, rest(old, next)) };
    });
  const species = (m: PartyMonDto) => speciesName(e.names.species, m.species);
  return (
    <section className={styles.section} aria-label={place} data-slot-details={slot}>
      <h3 className={styles.sectionTitle}>
        {place} <span className={styles.count}>{species(mon)}</span>
      </h3>
      <div className={styles.columns}>
        <Column title="Pokémon">
          <Row label="Art">
            <PickField
              label={`Art von ${place}`}
              text={species(mon)}
              choice="species"
              only={isEvolutionTarget}
              disabled={off}
              onPick={(v) => setMon((m) => ({ ...m, species: v, form: m.form < formCount(v) ? m.form : 0 }), (o, n) => `${species(o)} → ${species(n)}`)}
            />
          </Row>
          <Row label="Level">
            <NumberField label="Level" value={mon.level} min={1} max={100} disabled={off} showBase={e.showBase} onCommit={(v) => setMon((m) => ({ ...m, level: v }), (o, n) => `Level ${o.level} → ${n.level}`)} />
          </Row>
          <Row label={strength.label}>
            <NumberField
              label={strength.label}
              value={strengthOf(mon, e.family)}
              bits={strength.bits}
              hint={strength.hint}
              disabled={off}
              showBase={e.showBase}
              onCommit={(v) => setMon((m) => withStrength(m, e.family, v), (o, n) => `Stärke ${strengthOf(o, e.family)} → ${strengthOf(n, e.family)}`)}
            />
          </Row>
          {hgss && (
            <>
              <Row label="Geschlecht">
                <SelectField
                  label="Geschlecht"
                  value={genderOf(mon)}
                  options={GENDER_OPTIONS}
                  disabled={off}
                  onChange={(v) => setMon((m) => ({ ...m, gender: v, ability: abilityOf(m) }), (o, n) => `Geschlecht ${optionText(GENDER_OPTIONS, genderOf(o))} → ${optionText(GENDER_OPTIONS, genderOf(n))}`)}
                />
              </Row>
              <Row label="Fähigkeit">
                <SelectField
                  label="Fähigkeit"
                  value={abilityOf(mon)}
                  options={ABILITY_OPTIONS}
                  disabled={off}
                  onChange={(v) => setMon((m) => ({ ...m, ability: v, gender: genderOf(m) }), (o, n) => `Fähigkeit ${optionText(ABILITY_OPTIONS, abilityOf(o))} → ${optionText(ABILITY_OPTIONS, abilityOf(n))}`)}
                />
              </Row>
            </>
          )}
          {(forms > 1 || mon.form !== 0) && (
            <Row label="Form">
              <NumberField label="Form" value={mon.form} min={0} max={Math.min(63, Math.max(forms - 1, mon.form))} disabled={off} onCommit={(v) => setMon((m) => ({ ...m, form: v }), (o, n) => `Form ${o.form} → ${n.form}`)} />
            </Row>
          )}
          <span>
            <Button
              kind="danger"
              size="s"
              disabled={off}
              onClick={() => {
                e.write((now, title) => {
                  const old = now.party[slot];
                  return old ? { patch: { TrainerParty: { party: now.party.filter((_, i) => i !== slot) } }, label: slotLabel(title, slot, `${species(old)} entfernt`) } : null;
                });
                onRemoved();
              }}
            >
              Pokémon entfernen
            </Button>
          </span>
        </Column>
        <Column title="Item und Attacken">
          {mon.item !== null && e.doc.format.items ? (
            <Row label="Gegenstand">
              <PickField
                label={`Gegenstand von ${place}`}
                text={itemName(e.names.items, mon.item)}
                choice="item"
                disabled={off}
                onPick={(v) => setMon((m) => ({ ...m, item: v }), (o, n) => `Gegenstand ${itemName(e.names.items, o.item ?? 0)} → ${itemName(e.names.items, n.item ?? 0)}`)}
              />
            </Row>
          ) : (
            <p className={styles.note}>Items tragen ist aus (Reiter Trainer): dieses Pokémon trägt nichts.</p>
          )}
          {mon.moves !== null && e.doc.format.moves ? (
            mon.moves.map((id, k) => (
              <Row key={k} label={`Attacke ${k + 1}`}>
                <PickField
                  label={`Attacke ${k + 1} von ${place}`}
                  text={moveText(e.names, id)}
                  choice="move"
                  disabled={off}
                  onPick={(v) =>
                    setMon(
                      (m) => ({ ...m, moves: (m.moves ?? [0, 0, 0, 0]).map((x, j) => (j === k ? v : x)) as [number, number, number, number] }),
                      (o, n) => `Attacke ${k + 1} ${moveText(e.names, o.moves?.[k] ?? 0)} → ${moveText(e.names, n.moves?.[k] ?? 0)}`,
                    )
                  }
                />
              </Row>
            ))
          ) : (
            <p className={styles.note}>Eigene Attacken sind aus (Reiter Trainer): das Spiel gibt die Attacken aus dem Lernset bis zu seinem Level.</p>
          )}
        </Column>
      </div>
    </section>
  );
}

/** Team: six cards (filled places and free ones), the details of the chosen place below; a free card adds a Pokémon through a picker. */
function TeamTab({ e }: { e: Editing }) {
  const [chosen, setChosen] = useState(0);
  const [adding, setAdding] = useState(false);
  const species = useChoices("species", adding, isEvolutionTarget);
  const party = e.doc.party;
  const slot = Math.min(chosen, Math.max(0, party.length - 1));
  const add = async (picked: number) => {
    const at = party.length;
    const level = party[at - 1]?.level ?? 5;
    // With own moves the new Pokémon gets the moves the game would give it (the learnset of its species up to its level).
    let moves: [number, number, number, number] | undefined;
    if (e.doc.format.moves) {
      try {
        const doc = await dataGet("species", picked);
        if (doc.kind === "species") moves = defaultMoves(doc.learnset, level);
      } catch {
        moves = undefined;
      }
    }
    e.write((now, title) =>
      now.party.length >= PARTY_MAX
        ? null
        : {
            patch: { TrainerParty: { party: [...now.party, newMon(picked, level, now.format, e.family, moves)] } },
            label: slotLabel(title, now.party.length, `${speciesName(e.names.species, picked)} hinzugefügt`),
          },
    );
    setChosen(at);
  };
  return (
    <>
      <div className={styles.team} role="group" aria-label="Team">
        {Array.from({ length: PARTY_MAX }, (_, i) => (
          <SlotCard key={i} e={e} slot={i} chosen={i === slot && i < party.length} onChoose={() => setChosen(i)} onAdd={() => setAdding(true)} />
        ))}
      </div>
      {party.length === 0 ? (
        <p className={styles.note}>Ohne Pokémon stürzt der Kampf ab: wähle einen freien Platz und eine Art.</p>
      ) : (
        <SlotDetails e={e} slot={slot} onRemoved={() => setChosen(Math.max(0, slot - 1))} />
      )}
      <ChoicePicker open={adding} onOpenChange={setAdding} title={`Pokémon für Platz ${party.length + 1} wählen`} items={species} onPick={(v) => void add(v)} />
    </>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- trainer

/** The battle texts: the three a single trainer needs (a missing one can be made), any other kind it has, the warning of the first. */
function Texts({ e }: { e: Editing }) {
  const [making, setMaking] = useState<ReadonlySet<number>>(new Set());
  const { doc } = e;
  const off = e.locked || doc.texts_locked !== null;
  const kinds = [...TEXT_KINDS.map((k) => k.kind), ...doc.texts.map((t) => t.kind).filter((k) => !TEXT_KINDS.some((t) => t.kind === k))];
  const setText = (kind: number, text: string) =>
    e.write((now, title) => {
      const old = now.texts.find((t) => t.kind === kind);
      if (old ? old.text === text : text === "") return null;
      return { patch: { TrainerTexts: { texts: [{ kind, text }] } }, label: `${title}: Text ${textKind(kind).word} ${old ? "geändert" : "angelegt"}` };
    });
  const first = doc.texts.find((t) => t.kind === 0);
  return (
    <Column title="Kampftexte" note="der Text vor dem Kampf endet auf \r (⏎)">
      {doc.texts_locked !== null && <Notice kind="warning">{`Kampftexte schreibgeschützt: Trainertexte passen nicht zur Tabelle (${doc.texts_locked}).`}</Notice>}
      {doc.problems.map((p, i) => (
        <Notice
          key={i}
          kind="warning"
          actions={isMarkProblem(p) && first && !off && !first.text.endsWith("\\r") ? [{ label: "⏎ anhängen", onClick: () => setText(0, `${first.text}\\r`) }] : undefined}
        >
          {problemText(p)}
        </Notice>
      ))}
      {kinds.map((kind) => {
        const { label } = textKind(kind);
        const text = doc.texts.find((t) => t.kind === kind);
        if (!text && !making.has(kind)) {
          return (
            <Row key={kind} label={label}>
              <Button kind="quiet" size="s" icon="plus" aria-label={`Text ${label} anlegen`} disabled={off} onClick={() => setMaking((all) => new Set([...all, kind]))}>
                Text anlegen
              </Button>
            </Row>
          );
        }
        return (
          <Row key={kind} label={label}>
            <TextField label={label} value={text?.text ?? ""} disabled={off} onCommit={(v) => setText(kind, v)} />
          </Row>
        );
      })}
    </Column>
  );
}

/** Trainer: class and name, battle type and AI, the team's format, four items, the battle texts. */
function TrainerTabBody({ e }: { e: Editing }) {
  const { doc, names } = e;
  const off = e.locked;
  const [nameProblem, setNameProblem] = useState<string | null>(null);
  const classes = useMemo<SelectOption<number>[]>(() => classOptions(names.classes, doc.class), [names.classes, doc.class]);
  const setField = (field: "class" | "ai" | "battle_type" | "item0" | "item1" | "item2" | "item3", value: (now: TrainerDoc) => number, words: (old: number, next: number) => string) =>
    e.write((now, title) => {
      const old = field === "class" ? now.class : field === "ai" ? now.ai : field === "battle_type" ? now.battle_type : now.items[Number(field.slice(4))]!;
      const next = value(now);
      return old === next ? null : { patch: { Trainer: { field, value: next } }, label: `${title}: ${words(old, next)}` };
    });
  const setName = (name: string) => {
    const problem = nameError(name);
    setNameProblem(problem);
    if (problem !== null) return;
    e.write((now, title) => (now.name === name ? null : { patch: { TrainerName: { name } }, label: `${title}: Name ${now.name} → ${name}` }));
  };
  const setFormat = (part: "moves" | "items", on: boolean) =>
    e.write((now, title) =>
      now.format[part] === on
        ? null
        : { patch: { TrainerFormat: { moves: part === "moves" ? on : null, items: part === "items" ? on : null } }, label: `${title}: ${part === "moves" ? "eigene Attacken" : "Items tragen"} ${on ? "an" : "aus"}` },
    );
  const lockOptions = <V extends string>(options: readonly { value: V; label: string; disabled?: boolean }[]) => (off ? options.map((o) => ({ ...o, disabled: true })) : options);
  return (
    <div className={styles.columns}>
      <Column title="Trainer">
        <Row label="Klasse">
          <SelectField label="Klasse" value={doc.class} options={classes} disabled={off} onChange={(v) => setField("class", () => v, (o, n) => `Klasse ${className(names.classes, o)} → ${className(names.classes, n)}`)} />
        </Row>
        <Row label="Name">
          <TextField label="Name" value={doc.name} disabled={off} hint={nameCount(doc.name)} error={nameProblem ?? undefined} onCommit={setName} />
        </Row>
        <Row label="Kampfart">
          <Segmented
            ariaLabel="Kampfart"
            size="s"
            options={lockOptions(BATTLE_OPTIONS)}
            value={isDouble(doc.battle_type) ? "double" : "single"}
            onChange={(v) => setField("battle_type", (now) => withDouble(now.battle_type, v === "double"), (o, n) => `Kampfart ${battleText(o)} → ${battleText(n)}`)}
          />
        </Row>
        <Row label="KI">
          <Segmented
            ariaLabel="KI"
            size="s"
            options={lockOptions(AI_OPTIONS)}
            value={aiLevel(doc.ai)}
            onChange={(level) => {
              const preset = AI_LEVELS.find((l) => l.value === level)?.ai;
              if (preset !== undefined && preset !== null) setField("ai", () => preset, (o, n) => `KI ${aiText(o)} → ${aiText(n)}`);
            }}
          />
        </Row>
        {aiLevel(doc.ai) === "custom" && <p className={styles.note}>{`Eigene KI ${aiText(doc.ai).replace("eigene ", "")}: die Bits stehen unter Tüfteln in Rohdaten.`}</p>}
      </Column>
      <Column title="Team-Format">
        <Switch label="Eigene Attacken" checked={doc.format.moves} disabled={off} onChange={(on) => setFormat("moves", on)} />
        <Switch label="Items tragen" checked={doc.format.items} disabled={off} onChange={(on) => setFormat("items", on)} />
        <p className={styles.note}>
          Einschalten gibt jedem Pokémon die Attacken, die das Spiel ihm gäbe (Lernset bis zu seinem Level); Ausschalten verwirft die eigenen Attacken. Items tragen aus verwirft die
          Items der Pokémon.
        </p>
        <h3 className={styles.columnTitle}>Items des Trainers</h3>
        {doc.items.map((id, n) => (
          <Row key={n} label={`Gegenstand ${n + 1}`}>
            <PickField
              label={`Gegenstand ${n + 1}`}
              text={itemName(names.items, id)}
              choice="item"
              disabled={off}
              onPick={(v) => setField(`item${n}` as "item0", () => v, (o, x) => `Gegenstand ${n + 1} ${itemName(names.items, o)} → ${itemName(names.items, x)}`)}
            />
          </Row>
        ))}
      </Column>
      <Texts e={e} />
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- raw

const RAW_LABELS: Readonly<Record<string, string>> = {
  props_len: "Länge der Eigenschaften",
  flags: "Flags",
  has_moves: "Eigene Attacken",
  has_items: "Items tragen",
  class: "Klasse",
  unknown: "Unbekannt",
  party_count: "Anzahl Pokémon",
  items: "Gegenstände",
  ai: "KI",
  battle_type: "Kampfart",
  party: "Team",
};

/** Both members as the store holds them (`u32 len | properties | party`), read again with every new document. */
function RawBytes({ index, doc }: { index: number; doc: TrainerDoc | null }) {
  const [raw, setRaw] = useState<{ raw: RawDto | null; error: string | null } | null>(null);
  useEffect(() => {
    let current = true;
    dataRaw("trainer", index).then(
      (r) => current && setRaw({ raw: r, error: null }),
      (err: unknown) => current && setRaw({ raw: null, error: errorText(err) }),
    );
    return () => {
      current = false;
    };
  }, [index, doc]);
  if (raw === null) return <p className={styles.note}>Lädt …</p>;
  if (raw.raw === null) return <p className={styles.note}>{`Nicht lesbar: ${raw.error ?? ""}`}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} path={raw.raw.path} member={raw.raw.member} labelOf={(name) => RAW_LABELS[name] ?? name} />;
}

/** Rohdaten (Tüfteln): the bytes of both members, all 32 AI bits (11 named) and the battle type as a number. */
function RawTab({ e, read }: { e: Editing; read: TrainerDoc | null }) {
  const setAi = (bit: number, on: boolean) =>
    e.write((now, title) => {
      const ai = withBit(now.ai, bit, on);
      return ai === now.ai ? null : { patch: { Trainer: { field: "ai", value: ai } }, label: `${title}: KI ${aiText(now.ai)} → ${aiText(ai)}` };
    });
  return (
    <div className={styles.columns}>
      <Column title="Bytes" note="Eigenschaften und Team">
        <RawBytes index={e.index} doc={read} />
      </Column>
      <Column title="KI-Bits" note={`Wert 0x${(e.doc.ai >>> 0).toString(16)}`}>
        <div className={styles.bits}>
          {Array.from({ length: 32 }, (_, bit) => (
            <div key={bit} data-ai-bit={bit}>
              <Checkbox label={bitLabel(bit)} checked={(e.doc.ai >>> bit) % 2 === 1} disabled={e.locked} onChange={(on) => setAi(bit, on)} />
            </div>
          ))}
        </div>
        <Row label="Kampfart (roh)">
          <NumberField
            label="Kampfart (roh)"
            value={e.doc.battle_type}
            bits={32}
            disabled={e.locked}
            showBase
            onCommit={(v) => e.write((now, title) => (now.battle_type === v ? null : { patch: { Trainer: { field: "battle_type", value: v } }, label: `${title}: Kampfart ${now.battle_type} → ${v}` }))}
          />
        </Row>
      </Column>
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- hand

/**
 * The editor (Hand) of a trainer: the tabs Team, Trainer and, while tinkering, Rohdaten. Every field commits on its own as one undo step
 * with a speaking label („Wanderer Jens: Platz 1 Level 12 → 14“), built from the trainer as the backend holds it when the edit runs. The
 * placeholder of the sight script and a trainer of the wrong length show their fields disabled.
 */
export function TrainerHand() {
  const { row, labelOf, names, selected, project, dirty } = useDataArea("pokemon");
  const tinkering = useShell().state.depth === "tuefteln";
  const { doc, read, error, editError, update } = useTrainer(selected);
  const [tab, setTab] = useState<TrainerTab>("team");
  const index = selected ?? 0;
  const family = project?.family ?? "Pt";

  const editing = useMemo<Editing | null>(() => {
    if (doc === null) return null;
    const locked = doc.placeholder || doc.locked !== null;
    return {
      index,
      doc,
      names,
      family,
      locked,
      showBase: tinkering,
      write: (make) => {
        if (locked) return;
        void update(index, (now) => (now.placeholder || now.locked !== null ? null : make(now, docTitle(now, names.classes, index))));
      },
    };
  }, [doc, index, names, family, tinkering, update]);

  const unsaved = dirty.some((a) => a.kind === "Trainer" && a.index === selected);
  const items = useMemo<TabItem[]>(
    () => [
      { id: "team", label: "Team", count: doc?.party.length, unsaved },
      { id: "trainer", label: "Trainer", unsaved },
      ...(tinkering ? [{ id: "raw", label: "Rohdaten" }] : []),
    ],
    [doc, unsaved, tinkering],
  );

  if (row === null || selected === null) return null;
  if (editing === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what="Dieser Trainer ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}</div>;
  }
  const shown: TrainerTab = tab === "raw" && !tinkering ? "team" : tab;
  const title = editing.doc.placeholder || editing.doc.locked !== null ? labelOf(row) : docTitle(editing.doc, names.classes, index);
  return (
    <div className={styles.hand} data-trainer={index}>
      {(editError !== null || editing.locked) && (
        <div className={styles.notices}>
          {editing.doc.placeholder && <Notice kind="info">{`${PLACEHOLDER_TEXT}: er hält die Nummer frei, die das Kampf-Script für „gesehen“ nutzt. Alle Felder sind gesperrt.`}</Notice>}
          {!editing.doc.placeholder && editing.doc.locked !== null && <Notice kind="warning">{`${lockReason(editing.doc.locked)}. Die Felder dieses Trainers sind gesperrt.`}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.handHead}>
        <ClassSprite cls={editing.locked ? null : editing.doc.class} name={title} size={48} />
        <span className={styles.handTitle}>{title}</span>
      </div>
      <Tabs items={items} value={shown} onChange={(id) => setTab(id as TrainerTab)} ariaLabel={`${title} bearbeiten`}>
        <div className={styles.tabBody}>
          {shown === "team" && <TeamTab e={editing} />}
          {shown === "trainer" && <TrainerTabBody e={editing} />}
          {shown === "raw" && <RawTab e={editing} read={read} />}
        </div>
      </Tabs>
    </div>
  );
}
