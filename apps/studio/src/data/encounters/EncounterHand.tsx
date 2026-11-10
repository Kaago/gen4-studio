import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { dataRaw, errorText } from "../../api";
import type { EncounterMethod, NameTablesDto, RawDto, SlotDto } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { depthAtLeast } from "../../ui/frame/shellState";
import { Button, EmptyState, HexBlock, InspectorRow, Notice, NumberField, SelectField, Switch, Table, Tabs } from "../../ui/primitives";
import type { PickerItem, TabItem, TableRow } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { ChoicePicker, useChoices } from "../pokedex/pickers";
import { isEvolutionTarget } from "../pokedex/pokedexModel";
import { speciesName } from "../trainer/trainerModel";
import { useDataArea } from "../useDataArea";
import type { DocEdit } from "../useDataDoc";
import {
  EAST,
  FORM_RATES,
  TIME_COLUMNS,
  UNOWN_OPTIONS,
  entryLabel,
  formText,
  isEmptySlot,
  methodTabs,
  methodWord,
  rateLabel,
  replacedByText,
  replacesText,
  shareText,
  slotColumns,
  slotLabel,
  slotText,
  unownText,
  usedEntries,
  usedSlots,
  warningText,
  whenText,
} from "./encounterModel";
import type { EncounterDoc, EncounterTab } from "./encounterModel";
import { useEncounter } from "./useEncounter";
import styles from "./Encounter.module.css";

type HandTab = EncounterTab | "raw";

/** What every part of the editor works with: the file as shown, its title for the labels, and how to write one edit. */
interface Editing {
  index: number;
  doc: EncounterDoc;
  title: string;
  names: NameTablesDto;
  family: string;
  /** A file of the wrong length: every field is disabled and nothing is written. */
  locked: boolean;
  showBase: boolean;
  /** Writes one edit as one undo step, built from the file as the backend holds it when the edit runs (`useDataDoc`); null: nothing. */
  write(make: (now: EncounterDoc) => DocEdit | null): void;
}

/** A slot field as the patch names it. */
type SlotField = "species" | "species_morning" | "species_night" | "level" | "min" | "max";

function slotValue(slot: SlotDto, field: SlotField): number {
  switch (field) {
    case "species":
      return slot.species;
    case "species_morning":
      return slot.species_morning ?? 0;
    case "species_night":
      return slot.species_night ?? 0;
    case "level":
    case "min":
      return slot.min;
    case "max":
      return slot.max;
  }
}

/** A species by name; 0 is „Keine“ (an empty slot or entry). */
const speciesText = (names: NameTablesDto, id: number, none: string) => (id === 0 ? none : speciesName(names.species, id));

/** A species chosen from the project's species (without forms and eggs), or „Keine“; the button names the field and its species. */
function SpeciesPick({ label, value, names, disabled, onPick }: { label: string; value: number; names: NameTablesDto; disabled: boolean; onPick: (species: number) => void }) {
  const { t } = useTranslation("data");
  const [open, setOpen] = useState(false);
  const species = useChoices("species", open, isEvolutionTarget);
  const items = useMemo(() => {
    const none: PickerItem = { id: 0, label: t("none"), tech: id4(0), description: t("encounterHand.slotEmpty") };
    return open ? [none, ...species] : [];
  }, [open, species, t]);
  const text = speciesText(names, value, t("none"));
  return (
    <>
      <Button kind="secondary" size="s" aria-label={t("pickers.pickAria", { label, current: text })} disabled={disabled} onClick={() => setOpen(true)}>
        <span className={styles.pickText}>{text}</span>
      </Button>
      <ChoicePicker open={open} onOpenChange={setOpen} title={t("pickers.pickTitle", { label })} items={items} onPick={onPick} />
    </>
  );
}

/** The warnings of one method, in German, above its table. */
function Warnings({ e, method }: { e: Editing; method: EncounterMethod }) {
  const own = e.doc.warnings.filter((w) => w.method === method);
  if (own.length === 0) return null;
  return (
    <div className={styles.notices}>
      {own.map((w, i) => (
        <div key={i} data-warning={i}>
          <Notice kind="warning">{warningText(w.text, (id) => speciesName(e.names.species, id))}</Notice>
        </div>
      ))}
    </div>
  );
}

/**
 * One method: its rate („0 = nie“; Platinum reads only the low byte) and its slots as a table (Platz · Pokémon · Level · Anteil · Ersetzt
 * durch). HeartGold's grass has three species columns and one level; water has a level range. Empty slots are valid and muted.
 */
function MethodTab({ e, method }: { e: Editing; method: EncounterMethod }) {
  const { t } = useTranslation("data");
  const none = t("none");
  const m = e.doc.methods.find((x) => x.method === method);
  if (!m) return null;
  const word = methodWord(method);
  const hgssGrass = e.family === "Hgss" && method === "grass";
  const off = e.locked;
  const setRate = (value: number) =>
    e.write((now) => {
      const old = now.methods.find((x) => x.method === method)?.rate;
      return old === undefined || old === value ? null : { patch: { Encounter: { method, slot: null, field: "rate", value } }, label: rateLabel(e.title, method, old, value) };
    });
  const setSlot = (slot: number, field: SlotField, value: number, words: (old: number) => string) =>
    e.write((now) => {
      const s = now.methods.find((x) => x.method === method)?.slots.find((x) => x.index === slot);
      if (!s) return null;
      const old = slotValue(s, field);
      return old === value ? null : { patch: { Encounter: { method, slot, field, value } }, label: slotLabel(e.title, method, slot, words(old)) };
    });
  const pickSpecies = (s: SlotDto, field: "species" | "species_morning" | "species_night", label: string) => {
    // HeartGold's grass names the column in the label („Gras Platz 1 nachts Hoothoot → Taubsi“).
    const column = hgssGrass ? `${TIME_COLUMNS.find((c) => c.field === field)?.word ?? ""} ` : "";
    return (
      <SpeciesPick
        label={t("encounterHand.labelOf", { label, slot: slotText(s.index) })}
        value={slotValue(s, field)}
        names={e.names}
        disabled={off}
        onPick={(v) => setSlot(s.index, field, v, (old) => `${column}${speciesText(e.names, old, none)} → ${speciesText(e.names, v, none)}`)}
      />
    );
  };
  const rows: TableRow[] = m.slots.map((s) => {
    const replaced = replacedByText(s.replaced_by);
    const cells: Record<string, ReactNode> = {
      slot: <span className={styles.mono}>{slotText(s.index)}</span>,
      level:
        method === "grass" ? (
          <NumberField label={t("encounterHand.levelOf", { slot: slotText(s.index) })} value={s.min} bits={8} disabled={off} onCommit={(v) => setSlot(s.index, "level", v, (old) => t("encounterHand.levelChange", { from: old, to: v }))} />
        ) : (
          <span className={styles.levels}>
            <NumberField label={t("encounterHand.minLevelOf", { slot: slotText(s.index) })} value={s.min} bits={8} disabled={off} onCommit={(v) => setSlot(s.index, "min", v, (old) => t("encounterHand.minLevelChange", { from: old, to: v }))} />
            <span className={styles.levelDash} aria-hidden="true">
              –
            </span>
            <NumberField label={t("encounterHand.maxLevelOf", { slot: slotText(s.index) })} value={s.max} bits={8} disabled={off} onCommit={(v) => setSlot(s.index, "max", v, (old) => t("encounterHand.maxLevelChange", { from: old, to: v }))} />
          </span>
        ),
      share: <span className={styles.mono}>{shareText(s.share)}</span>,
      replaced: replaced === "" ? <span className={styles.muted}>—</span> : <span className={styles.replaced}>{replaced}</span>,
    };
    if (hgssGrass) for (const c of TIME_COLUMNS) cells[c.field] = pickSpecies(s, c.field, c.label);
    else cells.species = pickSpecies(s, "species", t("encounterModel.columns.pokemon"));
    return { key: String(s.index), cells, attrs: { "data-slot": String(s.index), ...(isEmptySlot(s) ? { "data-empty": "" } : {}) } };
  });
  return (
    <>
      <div className={styles.rateRow}>
        <InspectorRow label={t("encounterHand.rate", { word })}>
          <NumberField
            label={t("encounterHand.rate", { word })}
            value={m.rate}
            min={0}
            max={255}
            disabled={off}
            showBase={e.showBase}
            hint={e.family === "Hgss" ? t("encounterHand.rateHintHgss") : t("encounterHand.rateHint")}
            onCommit={setRate}
          />
        </InspectorRow>
      </div>
      <Warnings e={e} method={method} />
      <div className={styles.slots}>
        <Table columns={slotColumns(e.family, method)} rows={rows} ariaLabel={t("encounterHand.slots", { word })} />
      </div>
    </>
  );
}

/** The replacement lists of the file (each entry with the slots it replaces) and Platinum's form rates and Unown table. */
function ReplacementsTab({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const none = t("none");
  const off = e.locked;
  const setEntry = (list: string, entry: number, value: number) =>
    e.write((now) => {
      const old = now.replacements.find((l) => l.list === list)?.slots.find((s) => s.index === entry)?.species;
      return old === undefined || old === value
        ? null
        : { patch: { Encounter: { method: list, slot: entry, field: "species", value } }, label: entryLabel(e.title, list, entry, `${speciesText(e.names, old, none)} → ${speciesText(e.names, value, none)}`) };
    });
  const setExtra = (field: string, value: number, words: (now: EncounterDoc) => string | null) =>
    e.write((now) => {
      const old = words(now);
      return old === null ? null : { patch: { EncounterExtra: { field, value } }, label: `${e.title}: ${old}` };
    });
  const rates = e.doc.extras.form_rates;
  const unown = e.doc.extras.unown_table;
  return (
    <div className={styles.lists}>
      {e.doc.replacements.length === 0 && <p className={styles.note}>{t("encounterHand.noLists")}</p>}
      {e.doc.replacements.map((l) => (
        <section key={l.list} className={styles.section} aria-label={l.label} data-list={l.list}>
          <h3 className={styles.sectionTitle}>
            {l.label} <span className={styles.count}>{whenText(l.when)}</span>
          </h3>
          <Table
            ariaLabel={l.label}
            columns={[
              { key: "entry", label: t("encounterHand.entry"), width: 72 },
              { key: "species", label: t("encounterModel.columns.pokemon") },
              { key: "replaces", label: t("encounterHand.replaces") },
              { key: "share", label: t("encounterModel.columns.share"), width: 72, align: "end" },
            ]}
            rows={l.slots.map((s) => ({
              key: String(s.index),
              attrs: { "data-entry": String(s.index), ...(s.species === 0 ? { "data-empty": "" } : {}) },
              cells: {
                entry: <span className={styles.mono}>{s.index + 1}</span>,
                species: <SpeciesPick label={t("encounterHand.listEntry", { label: l.label, entry: s.index + 1 })} value={s.species} names={e.names} disabled={off} onPick={(v) => setEntry(l.list, s.index, v)} />,
                replaces: <span className={styles.replaced}>{replacesText(s.replaces)}</span>,
                share: <span className={styles.mono}>{shareText(s.share)}</span>,
              },
            }))}
          />
        </section>
      ))}
      {rates !== null && (
        <section className={styles.section} aria-label={t("encounterHand.forms")}>
          <h3 className={styles.sectionTitle}>
            {t("encounterHand.forms")} <span className={styles.count}>{t("encounterHand.formsNote")}</span>
          </h3>
          {FORM_RATES.map((f) => (
            <Switch
              key={f.field}
              label={t("encounterHand.eastLabel", { name: f.label })}
              checked={rates[f.index] !== 0}
              disabled={off}
              onChange={(on) => {
                const value = on ? EAST : 0;
                setExtra(f.field, value, (now) => {
                  const old = now.extras.form_rates?.[f.index];
                  // The raw value stays until the switch is turned: „Ost“ over any other non-zero value writes nothing.
                  return old === undefined || (old !== 0) === on ? null : `${f.label} ${formText(old)} → ${formText(value)}`;
                });
              }}
            />
          ))}
          {unown !== null && (
            <InspectorRow label={t("encounterHand.unownTable")}>
              <SelectField
                label={t("encounterHand.unownTable")}
                value={unown}
                options={UNOWN_OPTIONS}
                disabled={off}
                onChange={(v) =>
                  setExtra("unown_table", v, (now) => {
                    const old = now.extras.unown_table;
                    return old === null || old === v ? null : t("encounterHand.unownChange", { from: unownText(old), to: unownText(v) });
                  })
                }
              />
            </InspectorRow>
          )}
        </section>
      )}
    </div>
  );
}

/** The member as the store holds it (Tüfteln), read again with every new document. */
function RawTab({ index, doc }: { index: number; doc: EncounterDoc | null }) {
  const { t } = useTranslation("data");
  const [raw, setRaw] = useState<{ raw: RawDto | null; error: string | null } | null>(null);
  useEffect(() => {
    let current = true;
    dataRaw("encounter", index).then(
      (r) => current && setRaw({ raw: r, error: null }),
      (err: unknown) => current && setRaw({ raw: null, error: errorText(err) }),
    );
    return () => {
      current = false;
    };
  }, [index, doc]);
  if (raw === null) return <p className={styles.note}>{t("loading")}</p>;
  if (raw.raw === null) return <p className={styles.note}>{t("notReadable", { error: raw.error ?? "" })}</p>;
  return <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} path={raw.raw.path} member={raw.raw.member} />;
}

/**
 * The editor (Hand) of an encounter file: a tab per method with the count of its used slots, „Ersatz“ with the replacement lists (and
 * Platinum's forms), and while tinkering the raw bytes. Every field commits on its own as one undo step with a speaking label („Route 201:
 * Gras-Rate 30 → 100“), built from the file as the backend holds it when the edit runs. Shares, replacements and warnings come from the
 * backend's answer.
 */
export function EncounterHand() {
  const { t } = useTranslation("data");
  const { row, labelOf, names, selected, project, dirty } = useDataArea("pokemon");
  const tinkering = depthAtLeast(useShell().viewDepth, "tuefteln");
  const { doc, read, error, editError, update } = useEncounter(selected);
  const [tab, setTab] = useState<HandTab>("grass");
  const index = selected ?? 0;
  const family = project?.family ?? "Pt";
  const title = row === null ? `${t("dataModel.fallbackWord.encounter")} ${id4(index)}` : labelOf(row);

  const editing = useMemo<Editing | null>(() => {
    if (doc === null) return null;
    const locked = doc.locked !== null;
    return {
      index,
      doc,
      title,
      names,
      family,
      locked,
      showBase: tinkering,
      write: (make) => {
        if (locked) return;
        void update(index, (now) => (now.locked !== null ? null : make(now)));
      },
    };
  }, [doc, index, title, names, family, tinkering, update]);

  const unsaved = dirty.some((a) => a.kind === "Encounter" && a.index === selected);
  const items = useMemo<TabItem[]>(() => {
    if (doc === null) return [];
    const tabs = methodTabs(family).filter((x) => x.id === "replacements" || doc.methods.some((m) => m.method === x.id));
    return [
      ...tabs.map((x) => {
        const method = doc.methods.find((m) => m.method === x.id);
        return { id: x.id, label: x.label, count: method ? usedSlots(method) : usedEntries(doc), unsaved };
      }),
      ...(tinkering ? [{ id: "raw", label: t("rawData") }] : []),
    ];
  }, [doc, family, unsaved, tinkering, t]);

  if (row === null || selected === null) return null;
  if (editing === null) {
    return <div className={styles.handEmpty}>{error !== null ? <EmptyState what={t("encounterHand.unreadable")} why={error} /> : <p className={styles.note}>{t("loading")}</p>}</div>;
  }
  const shown: HandTab = items.some((x) => x.id === tab) ? tab : "grass";
  return (
    <div className={styles.hand} data-encounter={index}>
      {(editError !== null || editing.locked) && (
        <div className={styles.notices}>
          {editing.locked && <Notice kind="warning">{t("encounterHand.lockedFields", { reason: lockReason(editing.doc.locked ?? "") })}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <Tabs items={items} value={shown} onChange={(id) => setTab(id as HandTab)} ariaLabel={t("encounterHand.edit", { title })}>
        <div className={styles.tabBody}>
          {shown === "replacements" && <ReplacementsTab e={editing} />}
          {shown === "raw" && <RawTab index={index} doc={read} />}
          {shown !== "replacements" && shown !== "raw" && <MethodTab e={editing} method={shown} />}
        </div>
      </Tabs>
    </div>
  );
}
