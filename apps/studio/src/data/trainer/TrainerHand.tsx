import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { dataGet, dataRaw, errorText } from "../../api";
import type { NameTablesDto, PartyMonDto, RawDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { depthAtLeast } from "../../ui/frame/shellState";
import { Button, Checkbox, EmptyState, HexBlock, InspectorRow, Notice, NumberField, PokemonCard, Segmented, SelectField, Switch, Tabs, TextField } from "../../ui/primitives";
import type { SelectOption, TabItem } from "../../ui/primitives";
import { lockReason } from "../dataModel";
import { monIcon, monIconKey, useIcon } from "../icons";
import { ChoicePicker, PickField, useChoices } from "../pokedex/pickers";
import { isEvolutionTarget, itemName, moveName } from "../pokedex/pokedexModel";
import { MessageEditor } from "../../workbench/text/MessageEditor";
import { useDataArea } from "../useDataArea";
import type { DocEdit } from "../useDataDoc";
import {
  ABILITY_OPTIONS,
  AI_LEVELS,
  GENDER_OPTIONS,
  PARTY_MAX,
  TEXT_KINDS,
  abilityOf,
  aiHex,
  aiLevel,
  aiOptions,
  aiText,
  battleChange,
  battleOptions,
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
  placeholderText,
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
export interface Editing {
  index: number;
  doc: TrainerDoc;
  names: NameTablesDto;
  family: string;
  /** The placeholder or a trainer of the wrong length: every field is disabled and nothing is written. */
  locked: boolean;
  showBase: boolean;
  /**
   * Writes one edit as one undo step, built from the trainer as the backend holds it when the edit runs (`useDataDoc`), with its title
   * ("Wanderer Jens") for the label; `make` returns null for nothing to write. A locked trainer writes nothing. Resolves to whether the
   * edit went through (nothing to write counts as through; a locked trainer's as refused).
   */
  write(make: (now: TrainerDoc, title: string) => DocEdit | null): Promise<boolean>;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** A column of the hand: a bold title with its grey note. */
export function Column({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
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
  const { t } = useTranslation("data");
  const mon = e.doc.party[slot];
  const species = mon?.species ?? 0;
  const icon = useIcon(mon ? monIconKey(species) : null, () => monIcon(species), true);
  if (!mon) {
    return <PokemonCard state="empty" slot={slot + 1} ariaLabel={t("trainerHand.slotEmpty", { slot: slot + 1 })} onClick={onAdd} disabled={e.locked} dataAttrs={{ "data-slot": slot }} />;
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
      ariaLabel={t("trainerHand.slotCard", { slot: slot + 1, name, level: mon.level })}
      onClick={onChoose}
      disabled={e.locked}
      dataAttrs={{ "data-slot": slot }}
    />
  );
}

/** A field of the place details, with the place in the field's name for the picker buttons. */
export function Row({ label, children }: { label: string; children: ReactNode }) {
  return <InspectorRow label={label}>{children}</InspectorRow>;
}

/**
 * The details of one place: species (a picker with type badges), level, strength, HeartGold's gender and ability, the form for species with
 * forms, the item with items on, four moves with own moves on, and removing the Pokémon. Each change writes the whole party as one step.
 */
function SlotDetails({ e, slot, narrating, onRemoved }: { e: Editing; slot: number; narrating: boolean; onRemoved: () => void }) {
  const { t } = useTranslation("data");
  const mon = e.doc.party[slot]!;
  const place = t("trainerHand.slot", { slot: slot + 1 });
  const moveText = (names: NameTablesDto, id: number) => (id === 0 ? t("none") : moveName(names.moves, id));
  const hgss = e.family === "Hgss";
  const strength = strengthField(e.family);
  const forms = formCount(mon.species);
  const off = e.locked;
  // The switches for items and own moves are in the Trainer tab of Bauen only.
  const switches = narrating ? t("trainerHand.switchesNarrating") : t("trainerHand.switches");
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
        <Column title={t("trainerHand.pokemon")}>
          <Row label={t("trainerHand.species")}>
            <PickField
              label={t("trainerHand.speciesOf", { slot: slot + 1 })}
              text={species(mon)}
              choice="species"
              only={isEvolutionTarget}
              disabled={off}
              onPick={(v) => setMon((m) => ({ ...m, species: v, form: m.form < formCount(v) ? m.form : 0 }), (o, n) => `${species(o)} → ${species(n)}`)}
            />
          </Row>
          <Row label={t("trainerHand.level")}>
            <NumberField label={t("trainerHand.level")} value={mon.level} min={1} max={100} disabled={off} showBase={e.showBase} onCommit={(v) => setMon((m) => ({ ...m, level: v }), (o, n) => t("trainerHand.levelChange", { from: o.level, to: n.level }))} />
          </Row>
          <Row label={strength.label}>
            <NumberField
              label={strength.label}
              value={strengthOf(mon, e.family)}
              bits={strength.bits}
              hint={strength.hint}
              disabled={off}
              showBase={e.showBase}
              onCommit={(v) => setMon((m) => withStrength(m, e.family, v), (o, n) => t("trainerHand.strengthChange", { from: strengthOf(o, e.family), to: strengthOf(n, e.family) }))}
            />
          </Row>
          {hgss && (
            <>
              <Row label={t("trainerHand.gender")}>
                <SelectField
                  label={t("trainerHand.gender")}
                  value={genderOf(mon)}
                  options={GENDER_OPTIONS}
                  disabled={off}
                  onChange={(v) => setMon((m) => ({ ...m, gender: v, ability: abilityOf(m) }), (o, n) => t("trainerHand.genderChange", { from: optionText(GENDER_OPTIONS, genderOf(o)), to: optionText(GENDER_OPTIONS, genderOf(n)) }))}
                />
              </Row>
              <Row label={t("trainerHand.ability")}>
                <SelectField
                  label={t("trainerHand.ability")}
                  value={abilityOf(mon)}
                  options={ABILITY_OPTIONS}
                  disabled={off}
                  onChange={(v) => setMon((m) => ({ ...m, ability: v, gender: genderOf(m) }), (o, n) => t("trainerHand.abilityChange", { from: optionText(ABILITY_OPTIONS, abilityOf(o)), to: optionText(ABILITY_OPTIONS, abilityOf(n)) }))}
                />
              </Row>
            </>
          )}
          {(forms > 1 || mon.form !== 0) && (
            <Row label={t("trainerHand.form")}>
              <NumberField label={t("trainerHand.form")} value={mon.form} min={0} max={Math.min(63, Math.max(forms - 1, mon.form))} disabled={off} onCommit={(v) => setMon((m) => ({ ...m, form: v }), (o, n) => t("trainerHand.formChange", { from: o.form, to: n.form }))} />
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
                  return old ? { patch: { TrainerParty: { party: now.party.filter((_, i) => i !== slot) } }, label: slotLabel(title, slot, t("trainerHand.removed", { species: species(old) })) } : null;
                });
                onRemoved();
              }}
            >
              {t("trainerHand.removeMon")}
            </Button>
          </span>
        </Column>
        <Column title={t("trainerHand.itemAndMoves")}>
          {mon.item !== null && e.doc.format.items ? (
            <Row label={t("trainerHand.item")}>
              <PickField
                label={t("trainerHand.itemOf", { slot: slot + 1 })}
                text={itemName(e.names.items, mon.item)}
                choice="item"
                disabled={off}
                onPick={(v) => setMon((m) => ({ ...m, item: v }), (o, n) => t("trainerHand.itemChange", { from: itemName(e.names.items, o.item ?? 0), to: itemName(e.names.items, n.item ?? 0) }))}
              />
            </Row>
          ) : (
            <p className={styles.note}>{t("trainerHand.itemsOff", { switches })}</p>
          )}
          {mon.moves !== null && e.doc.format.moves ? (
            mon.moves.map((id, k) => (
              <Row key={k} label={t("trainerHand.move", { n: k + 1 })}>
                <PickField
                  label={t("trainerHand.moveOf", { n: k + 1, slot: slot + 1 })}
                  text={moveText(e.names, id)}
                  choice="move"
                  disabled={off}
                  onPick={(v) =>
                    setMon(
                      (m) => ({ ...m, moves: (m.moves ?? [0, 0, 0, 0]).map((x, j) => (j === k ? v : x)) as [number, number, number, number] }),
                      (o, n) => t("trainerHand.moveChange", { n: k + 1, from: moveText(e.names, o.moves?.[k] ?? 0), to: moveText(e.names, n.moves?.[k] ?? 0) }),
                    )
                  }
                />
              </Row>
            ))
          ) : (
            <p className={styles.note}>{t("trainerHand.movesOff", { switches })}</p>
          )}
        </Column>
      </div>
    </section>
  );
}

/**
 * Team: six cards (filled places and free ones), the details of the chosen place below; a free card adds a Pokémon through a picker.
 * `narrating`: the hand of Erzählen, whose notes point to the switches of Bauen.
 */
export function TeamTab({ e, narrating = false }: { e: Editing; narrating?: boolean }) {
  const { t } = useTranslation("data");
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
            label: slotLabel(title, now.party.length, t("trainerHand.added", { species: speciesName(e.names.species, picked) })),
          },
    );
    setChosen(at);
  };
  return (
    <>
      <div className={styles.team} role="group" aria-label={t("trainerHand.party")}>
        {Array.from({ length: PARTY_MAX }, (_, i) => (
          <SlotCard key={i} e={e} slot={i} chosen={i === slot && i < party.length} onChoose={() => setChosen(i)} onAdd={() => setAdding(true)} />
        ))}
      </div>
      {party.length === 0 ? (
        <p className={styles.note}>{t("trainerHand.emptyParty")}</p>
      ) : (
        <SlotDetails e={e} slot={slot} narrating={narrating} onRemoved={() => setChosen(Math.max(0, slot - 1))} />
      )}
      <ChoicePicker open={adding} onOpenChange={setAdding} title={t("trainerHand.pickFor", { slot: party.length + 1 })} items={species} onPick={(v) => void add(v)} />
    </>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- trainer

/**
 * The battle texts: the three a single trainer needs (a missing one can be made), any other kind it has, the warning of the first.
 * `narrating`: Erzählen writes them in the message editor (a new page with Shift+Enter, the codes as chips) and names no raw codes.
 */
export function Texts({ e, narrating = false }: { e: Editing; narrating?: boolean }) {
  const { t } = useTranslation("data");
  const [making, setMaking] = useState<ReadonlySet<number>>(new Set());
  const { doc } = e;
  const off = e.locked || doc.texts_locked !== null;
  const kinds = [...TEXT_KINDS.map((k) => k.kind), ...doc.texts.map((x) => x.kind).filter((k) => !TEXT_KINDS.some((x) => x.kind === k))];
  const setText = (kind: number, text: string) =>
    e.write((now, title) => {
      const old = now.texts.find((x) => x.kind === kind);
      if (old ? old.text === text : text === "") return null;
      const word = textKind(kind).word;
      return { patch: { TrainerTexts: { texts: [{ kind, text }] } }, label: `${title}: ${old ? t("trainerHand.textChanged", { word }) : t("trainerHand.textCreated", { word })}` };
    });
  const first = doc.texts.find((x) => x.kind === 0);
  return (
    <Column title={t("trainerHand.battleTexts")} note={narrating ? t("trainerHand.textNoteNarrating") : t("trainerHand.textNote")}>
      {doc.texts_locked !== null && <Notice kind="warning">{t("trainerHand.textsLocked", { reason: doc.texts_locked })}</Notice>}
      {doc.problems.map((p, i) => (
        <Notice
          key={i}
          kind="warning"
          actions={isMarkProblem(p) && first && !off && !first.text.endsWith("\\r") ? [{ label: t("trainerHand.appendReturn"), onClick: () => setText(0, `${first.text}\\r`) }] : undefined}
        >
          {problemText(p)}
        </Notice>
      ))}
      {kinds.map((kind) => {
        const { label } = textKind(kind);
        const text = doc.texts.find((x) => x.kind === kind);
        if (!text && !making.has(kind)) {
          return (
            <Row key={kind} label={label}>
              <Button kind="quiet" size="s" icon="plus" aria-label={t("trainerHand.createText", { label })} disabled={off} onClick={() => setMaking((all) => new Set([...all, kind]))}>
                {t("trainerHand.createTextShort")}
              </Button>
            </Row>
          );
        }
        return (
          <Row key={kind} label={label}>
            {narrating ? (
              <MessageEditor label={label} value={text?.text ?? ""} disabled={off} onCommit={(v) => setText(kind, v)} />
            ) : (
              <TextField label={label} value={text?.text ?? ""} disabled={off} onCommit={(v) => void setText(kind, v)} />
            )}
          </Row>
        );
      })}
    </Column>
  );
}

type TrainerField = "class" | "ai" | "battle_type" | "item0" | "item1" | "item2" | "item3";

/** Writes one number field of the trainer, labelled "<title>: <words>"; the same value writes nothing. */
function setField(e: Editing, field: TrainerField, value: (now: TrainerDoc) => number, words: (old: number, next: number) => string) {
  e.write((now, title) => {
    const old = field === "class" ? now.class : field === "ai" ? now.ai : field === "battle_type" ? now.battle_type : now.items[Number(field.slice(4))]!;
    const next = value(now);
    return old === next ? null : { patch: { Trainer: { field, value: next } }, label: `${title}: ${words(old, next)}` };
  });
}

/** The class as a select over `options` (Bauen: every class of the bank; Erzählen: the named ones). */
export function ClassRow({ e, options }: { e: Editing; options: SelectOption<number>[] }) {
  const { t } = useTranslation("data");
  const { names } = e;
  return (
    <Row label={t("trainerHand.class")}>
      <SelectField
        label={t("trainerHand.class")}
        value={e.doc.class}
        options={options}
        disabled={e.locked}
        onChange={(v) => setField(e, "class", () => v, (o, n) => t("trainerHand.classChange", { from: className(names.classes, o), to: className(names.classes, n) }))}
      />
    </Row>
  );
}

/** The name (at most 7 characters): a longer one is refused with the reason and writes nothing. */
export function NameRow({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const [nameProblem, setNameProblem] = useState<string | null>(null);
  const setName = (name: string) => {
    const problem = nameError(name);
    setNameProblem(problem);
    if (problem !== null) return;
    e.write((now, title) => (now.name === name ? null : { patch: { TrainerName: { name } }, label: `${title}: ${t("trainerHand.nameChange", { from: now.name, to: name })}` }));
  };
  return (
    <Row label={t("trainerHand.name")}>
      <TextField label={t("trainerHand.name")} value={e.doc.name} disabled={e.locked} hint={nameCount(e.doc.name)} error={nameProblem ?? undefined} onCommit={setName} />
    </Row>
  );
}

/** Switches the double battle bit of the battle type, labelled like the segment of Bauen („Kampfart Einzel → Doppel“). */
export function setDouble(e: Editing, on: boolean) {
  setField(e, "battle_type", (now) => withDouble(now.battle_type, on), (o, n) => battleChange(battleText(o), battleText(n)));
}

/** Trainer: class and name, battle type and AI, the team's format, four items, the battle texts. */
function TrainerTabBody({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const { doc, names } = e;
  const off = e.locked;
  const classes = useMemo<SelectOption<number>[]>(() => classOptions(names.classes, doc.class), [names.classes, doc.class]);
  const setFormat = (part: "moves" | "items", on: boolean) =>
    e.write((now, title) =>
      now.format[part] === on
        ? null
        : { patch: { TrainerFormat: { moves: part === "moves" ? on : null, items: part === "items" ? on : null } }, label: `${title}: ${part === "moves" ? t("trainerHand.partMoves") : t("trainerHand.partItems")} ${on ? t("trainerHand.on") : t("trainerHand.off")}` },
    );
  const lockOptions = <V extends string>(options: readonly { value: V; label: string; disabled?: boolean }[]) => (off ? options.map((o) => ({ ...o, disabled: true })) : options);
  return (
    <div className={styles.columns}>
      <Column title={t("trainerHand.trainer")}>
        <ClassRow e={e} options={classes} />
        <NameRow e={e} />
        <Row label={t("trainerHand.battleType")}>
          <Segmented
            ariaLabel={t("trainerHand.battleType")}
            size="s"
            options={lockOptions(battleOptions())}
            value={isDouble(doc.battle_type) ? "double" : "single"}
            onChange={(v) => setDouble(e, v === "double")}
          />
        </Row>
        <Row label={t("trainerHand.ai")}>
          <Segmented
            ariaLabel={t("trainerHand.ai")}
            size="s"
            options={lockOptions(aiOptions())}
            value={aiLevel(doc.ai)}
            onChange={(level) => {
              const preset = AI_LEVELS.find((l) => l.value === level)?.ai;
              if (preset !== undefined && preset !== null) setField(e, "ai", () => preset, (o, n) => t("trainerHand.aiChange", { from: aiText(o), to: aiText(n) }));
            }}
          />
        </Row>
        {aiLevel(doc.ai) === "custom" && <p className={styles.note}>{t("trainerHand.customAi", { hex: aiHex(doc.ai) })}</p>}
      </Column>
      <Column title={t("trainerHand.partyFormat")}>
        <Switch label={t("trainerHand.ownMoves")} checked={doc.format.moves} disabled={off} onChange={(on) => setFormat("moves", on)} />
        <Switch label={t("trainerHand.holdItems")} checked={doc.format.items} disabled={off} onChange={(on) => setFormat("items", on)} />
        <p className={styles.note}>{t("trainerHand.formatNote")}</p>
        <h3 className={styles.columnTitle}>{t("trainerHand.trainerItems")}</h3>
        {doc.items.map((id, n) => (
          <Row key={n} label={t("trainerHand.trainerItem", { n: n + 1 })}>
            <PickField
              label={t("trainerHand.trainerItem", { n: n + 1 })}
              text={itemName(names.items, id)}
              choice="item"
              disabled={off}
              onPick={(v) => setField(e, `item${n}` as "item0", () => v, (o, x) => t("trainerHand.trainerItemChange", { n: n + 1, from: itemName(names.items, o), to: itemName(names.items, x) }))}
            />
          </Row>
        ))}
      </Column>
      <Texts e={e} />
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- raw

/** The fields of the raw trainer bytes by the kernel's name, to the key of their label in the catalog. */
const RAW_LABEL_KEYS: Readonly<Record<string, "propsLen" | "flags" | "hasMoves" | "hasItems" | "class" | "unknown" | "partyCount" | "items" | "ai" | "battleType" | "party">> = {
  props_len: "propsLen",
  flags: "flags",
  has_moves: "hasMoves",
  has_items: "hasItems",
  class: "class",
  unknown: "unknown",
  party_count: "partyCount",
  items: "items",
  ai: "ai",
  battle_type: "battleType",
  party: "party",
};

/** Both members as the store holds them (`u32 len | properties | party`), read again with every new document. */
function RawBytes({ index, doc }: { index: number; doc: TrainerDoc | null }) {
  const { t } = useTranslation("data");
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
  if (raw === null) return <p className={styles.note}>{t("loading")}</p>;
  if (raw.raw === null) return <p className={styles.note}>{t("notReadable", { error: raw.error ?? "" })}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} path={raw.raw.path} member={raw.raw.member} labelOf={(name) => (RAW_LABEL_KEYS[name] === undefined ? name : t(`trainerHand.raw.${RAW_LABEL_KEYS[name]}`))} />;
}

/** Rohdaten (Tüfteln): the bytes of both members, all 32 AI bits (11 named) and the battle type as a number. */
function RawTab({ e, read }: { e: Editing; read: TrainerDoc | null }) {
  const { t } = useTranslation("data");
  const setAi = (bit: number, on: boolean) =>
    e.write((now, title) => {
      const ai = withBit(now.ai, bit, on);
      return ai === now.ai ? null : { patch: { Trainer: { field: "ai", value: ai } }, label: `${title}: ${t("trainerHand.aiChange", { from: aiText(now.ai), to: aiText(ai) })}` };
    });
  return (
    <div className={styles.columns}>
      <Column title={t("trainerHand.bytes")} note={t("trainerHand.bytesNote")}>
        <RawBytes index={e.index} doc={read} />
      </Column>
      <Column title={t("trainerHand.aiBits")} note={t("trainerHand.aiBitsNote", { hex: aiHex(e.doc.ai) })}>
        <div className={styles.bits}>
          {Array.from({ length: 32 }, (_, bit) => (
            <div key={bit} data-ai-bit={bit}>
              <Checkbox label={bitLabel(bit)} checked={(e.doc.ai >>> bit) % 2 === 1} disabled={e.locked} onChange={(on) => setAi(bit, on)} />
            </div>
          ))}
        </div>
        <Row label={t("trainerHand.battleTypeRaw")}>
          <NumberField
            label={t("trainerHand.battleTypeRaw")}
            value={e.doc.battle_type}
            bits={32}
            disabled={e.locked}
            showBase
            onCommit={(v) => e.write((now, title) => (now.battle_type === v ? null : { patch: { Trainer: { field: "battle_type", value: v } }, label: `${title}: ${battleChange(now.battle_type, v)}` }))}
          />
        </Row>
      </Column>
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------------- hand

/**
 * The chosen trainer as both hands edit it: its document, how an edit is written (none for the placeholder or a trainer of the wrong
 * length) and whether it has unsaved changes. `showBase`: number fields show their hex base (Tüfteln).
 */
export function useTrainerEditing(showBase: boolean) {
  const { row, labelOf, names, selected, project, dirty } = useDataArea("pokemon");
  const { doc, read, error, editError, update } = useTrainer(selected);
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
      showBase,
      write: (make) => (locked ? Promise.resolve(false) : update(index, (now) => (now.placeholder || now.locked !== null ? null : make(now, docTitle(now, names.classes, index))))),
    };
  }, [doc, index, names, family, showBase, update]);

  const unsaved = dirty.some((a) => a.kind === "Trainer" && a.index === selected);
  /** The hand's title: "Wanderer Jens", a locked trainer by its row. */
  const title = editing === null || row === null ? "" : editing.locked ? labelOf(row) : docTitle(editing.doc, names.classes, index);
  return { row, selected, index, editing, read, error, editError, unsaved, title };
}

/** What a hand shows while its trainer has no document: why it cannot be read, or that it is being read. */
export function HandWaiting({ error }: { error: string | null }) {
  const { t } = useTranslation("data");
  return <div className={styles.handEmpty}>{error !== null ? <EmptyState what={t("trainerHand.unreadable")} why={error} /> : <p className={styles.note}>{t("loading")}</p>}</div>;
}

/** The top of a hand: why its fields are locked, a failed edit, and the class sprite beside the title. */
export function HandTop({ e, editError, title }: { e: Editing; editError: string | null; title: string }) {
  const { t } = useTranslation("data");
  return (
    <>
      {(editError !== null || e.locked) && (
        <div className={styles.notices}>
          {e.doc.placeholder && <Notice kind="info">{t("trainerHand.placeholderNote", { placeholder: placeholderText() })}</Notice>}
          {!e.doc.placeholder && e.doc.locked !== null && <Notice kind="warning">{t("trainerHand.lockedFields", { reason: lockReason(e.doc.locked) })}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <div className={styles.handHead}>
        <ClassSprite cls={e.locked ? null : e.doc.class} name={title} size={48} />
        <span className={styles.handTitle}>{title}</span>
      </div>
    </>
  );
}

/**
 * The editor (Hand) of a trainer: the tabs Team, Trainer and, while tinkering, Rohdaten. Every field commits on its own as one undo step
 * with a speaking label („Wanderer Jens: Platz 1 Level 12 → 14“), built from the trainer as the backend holds it when the edit runs. The
 * placeholder of the sight script and a trainer of the wrong length show their fields disabled.
 */
export function TrainerHand() {
  const { t } = useTranslation("data");
  const tinkering = depthAtLeast(useShell().viewDepth, "tuefteln");
  const { row, selected, index, editing, read, error, editError, unsaved, title } = useTrainerEditing(tinkering);
  const [tab, setTab] = useState<TrainerTab>("team");
  const count = editing?.doc.party.length;
  const items = useMemo<TabItem[]>(
    () => [
      { id: "team", label: t("trainerHand.party"), count, unsaved },
      { id: "trainer", label: t("trainerHand.trainer"), unsaved },
      ...(tinkering ? [{ id: "raw", label: t("rawData") }] : []),
    ],
    [count, unsaved, tinkering, t],
  );

  if (row === null || selected === null) return null;
  if (editing === null) return <HandWaiting error={error} />;
  const shown: TrainerTab = tab === "raw" && !tinkering ? "team" : tab;
  return (
    <div className={styles.hand} data-trainer={index}>
      <HandTop e={editing} editError={editError} title={title} />
      <Tabs items={items} value={shown} onChange={(id) => setTab(id as TrainerTab)} ariaLabel={t("trainerHand.edit", { title })}>
        <div className={styles.tabBody}>
          {shown === "team" && <TeamTab e={editing} />}
          {shown === "trainer" && <TrainerTabBody e={editing} />}
          {shown === "raw" && <RawTab e={editing} read={read} />}
        </div>
      </Tabs>
    </div>
  );
}
