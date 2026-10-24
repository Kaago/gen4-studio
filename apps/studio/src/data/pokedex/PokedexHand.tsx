import { useCallback, useEffect, useMemo, useState } from "react";
import type { FocusEvent } from "react";
import type { DataPatch, NameTablesDto, PersonalField, SpeciesTab } from "../../api";
import { useShell } from "../../ui/frame/ShellContext";
import { EmptyState, Notice, Tabs } from "../../ui/primitives";
import type { TabItem } from "../../ui/primitives";
import { useDataArea } from "../useDataArea";
import { AbilitiesTab } from "./AbilitiesTab";
import { EvolutionTab } from "./EvolutionTab";
import { LearnsetTab } from "./LearnsetTab";
import { MachinesTab } from "./MachinesTab";
import { TABS, TAB_ASSETS, lockText, lockedParts, personalLabel } from "./pokedexModel";
import { RawTab } from "./RawTab";
import { useSpecies } from "./useSpecies";
import type { SpeciesDoc, SpeciesEdit } from "./useSpecies";
import { ValuesTab } from "./ValuesTab";
import styles from "./Pokedex.module.css";

/** A personal field that holds a number (all but `flip`). */
export type NumericField = Exclude<PersonalField, "flip">;

/** What every tab of the editor works with: the species, its document, and how to write it. */
export interface Editing {
  index: number;
  /** The species' name as the list shows it; undo labels start with it ("Staravia: Angriff 75 → 85"). */
  name: string;
  doc: SpeciesDoc;
  names: NameTablesDto;
  /** Number fields show the Dez | Hex switch (Tüfteln). */
  showBase: boolean;
  /** Which parts have the wrong length: their fields are locked. */
  locked: { personal: boolean; learnset: boolean; evolution: boolean };
  /** Writes one patch to this species as one undo step. */
  edit(patch: DataPatch, label: string): Promise<boolean>;
  /** Writes one patch built from the species as the edits before it left it (see `useSpecies().update`). */
  update(make: (doc: SpeciesDoc) => SpeciesEdit | null): Promise<boolean>;
  /** Writes one personal field, labelled with its old and new value. */
  setPersonal(field: NumericField, value: number): void;
}

/**
 * The editor (Hand) of a species: the tabs Werte, Fähigkeiten und Typen, Lernset, Entwicklung, TM/VM and, while tinkering, Rohdaten. A
 * tab's dot says its part has unsaved changes; a link with a tab opens that tab. Every field commits on its own as one undo step; a
 * refusal of the backend shows above the tabs, a part with the wrong length is named there and its fields are locked.
 */
export function PokedexHand() {
  const { row, labelOf, names, selected, dirty, target } = useDataArea("pokemon");
  const tinkering = useShell().state.depth === "tuefteln";
  const { doc, error, editError, edit, update } = useSpecies(selected);
  const [tab, setTab] = useState<SpeciesTab>("values");
  // The field focused last, for the raw data's mark (its key in the field map: "attack", "method0").
  const [lastField, setLastField] = useState<string | undefined>(undefined);

  // A link that names a tab of the editor shows it.
  useEffect(() => {
    if (target !== null && target.sub === "pokedex" && target.tab) setTab(target.tab);
  }, [target]);

  const name = row === null ? "" : labelOf(row);
  const index = selected ?? 0;
  const shown: SpeciesTab = tab === "raw" && !tinkering ? "values" : tab;

  // The label names the value the field had when the write comes (an edit before it may have changed it).
  const setPersonal = useCallback(
    (field: NumericField, value: number) =>
      void update(index, (now) => (now.personal[field] === value ? null : { patch: { Personal: { field, value } }, label: personalLabel(name, field, now.personal[field], value, names) })),
    [update, index, name, names],
  );

  const editing = useMemo<Editing | null>(() => {
    if (doc === null) return null;
    return {
      index,
      name,
      doc,
      names,
      showBase: tinkering,
      locked: lockedParts(doc),
      edit: (patch, label) => edit(index, patch, label),
      update: (make) => update(index, make),
      setPersonal,
    };
  }, [doc, index, name, names, tinkering, edit, update, setPersonal]);

  const items = useMemo<TabItem[]>(
    () =>
      TABS.filter((t) => t.id !== "raw" || tinkering).map((t) => ({
        id: t.id,
        label: t.label,
        count: t.id === "learnset" ? doc?.learnset.length : t.id === "machines" ? doc?.tm.filter(Boolean).length : undefined,
        unsaved: dirty.some((a) => a.index === selected && TAB_ASSETS[t.id].includes(a.kind)),
      })),
    [tinkering, doc, dirty, selected],
  );

  const onFocus = (event: FocusEvent<HTMLElement>) => {
    const field = (event.target as HTMLElement).closest<HTMLElement>("[data-field]")?.dataset.field;
    if (field !== undefined) setLastField(field);
  };

  if (row === null || selected === null) return null;
  if (editing === null) {
    return (
      <div className={styles.handEmpty}>
        {error !== null ? <EmptyState what="Diese Art ist nicht lesbar." why={error} /> : <p className={styles.note}>Lädt …</p>}
      </div>
    );
  }
  return (
    <div className={styles.hand} onFocus={onFocus}>
      {(editError !== null || doc?.locked) && (
        <div className={styles.notices}>
          {doc?.locked && <Notice kind="warning">{`${lockText(doc.locked, doc.locked_asset)}. Die Felder dieses Teils sind gesperrt.`}</Notice>}
          {editError !== null && <Notice kind="error">{editError}</Notice>}
        </div>
      )}
      <Tabs items={items} value={shown} onChange={(id) => setTab(id as SpeciesTab)} ariaLabel={`${name} bearbeiten`}>
        <div className={styles.tabBody}>
          {shown === "values" && <ValuesTab e={editing} tinkering={tinkering} />}
          {shown === "abilities" && <AbilitiesTab e={editing} />}
          {shown === "learnset" && <LearnsetTab e={editing} />}
          {shown === "evolution" && <EvolutionTab e={editing} />}
          {shown === "machines" && <MachinesTab e={editing} />}
          {shown === "raw" && <RawTab e={editing} highlight={lastField} />}
        </div>
      </Tabs>
    </div>
  );
}
