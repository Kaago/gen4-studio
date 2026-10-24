import { useState } from "react";
import { ListPane } from "../../frame/ListPane";
import { CheckList, FilterChips, HexBlock, NumberField, PokemonCard, Segmented, Sprite, StatBar, Table, ToggleGrid, TypeBadge, pokemonTypes } from "../../primitives";
import type { HexFieldSpan, TableRow, ToggleCell } from "../../primitives";
import styles from "../Gallery.module.css";

// Staravia's base stats against a made-up average of all species; Angriff is changed from 75.
const stats: { label: string; value: number; average: number; deviates?: boolean }[] = [
  { label: "KP", value: 55, average: 68 },
  { label: "Angriff", value: 85, average: 76, deviates: true },
  { label: "Verteidigung", value: 50, average: 72 },
  { label: "Spezial-Angriff", value: 40, average: 70 },
  { label: "Spezial-Verteidigung", value: 40, average: 70 },
  { label: "Initiative", value: 80, average: 66 },
];

const typeOptions = pokemonTypes.map((t) => ({ value: String(t.id), label: t.name, type: t.id }));

const learnset: [number, string][] = [
  [1, "Tackle"],
  [1, "Heuler"],
  [5, "Ruckzuckhieb"],
  [9, "Flügelschlag"],
  [13, "Doppelteam"],
];

// TM38 to TM49 with a few names; TM42 without (no machine table), TM45 locked.
const machineNames: Record<number, string> = { 38: "Feuerschlag", 40: "Aero-Ass", 41: "Folterknecht", 43: "Geheimpower", 44: "Erholung" };

// An item entry: price 300, then hold effect, parameter and fling power; 36 bytes.
const itemBytes = [0x2c, 0x01, 0x05, 0x14, 0x1e, 0x00, 0x00, 0x01, ...Array.from({ length: 28 }, () => 0)];
const itemFields: HexFieldSpan[] = [
  { name: "price", offset: 0, len: 2, kind: "u16" },
  { name: "hold_effect", offset: 2, len: 1, kind: "u8" },
  { name: "hold_param", offset: 3, len: 1, kind: "u8" },
  { name: "fling_power", offset: 4, len: 1, kind: "u8" },
];
const itemLabels: Record<string, string> = { price: "Preis", hold_effect: "Trageeffekt", hold_param: "Trageparameter", fling_power: "Schleuderstärke" };

function LearnsetTable() {
  const [levels, setLevels] = useState(learnset.map(([level]) => level));
  const [selected, setSelected] = useState("r3");
  const [deleted, setDeleted] = useState<string[]>([]);
  const rows: TableRow[] = learnset
    .map(([, move], i): TableRow => ({
      key: `r${i}`,
      attrs: { "data-move": String(i) },
      cells: {
        level: <NumberField label={`Level von ${move}`} value={levels[i]!} min={1} max={100} onCommit={(v) => setLevels((l) => l.map((x, j) => (j === i ? v : x)))} />,
        move,
        type: <TypeBadge type={move === "Flügelschlag" ? 2 : 0} size="s" />,
      },
    }))
    .filter((row) => !deleted.includes(row.key));
  return (
    <Table
      ariaLabel="Lernset"
      columns={[
        { key: "level", label: "Level", width: 96 },
        { key: "move", label: "Attacke" },
        { key: "type", label: "Typ", width: 88 },
      ]}
      rows={rows}
      selected={selected}
      onSelect={setSelected}
      onDeleteRow={(key) => setDeleted((d) => [...d, key])}
      footer={`${rows.length} von 20 Attacken`}
    />
  );
}

function Machines() {
  const [on, setOn] = useState<Set<string>>(new Set(["tm40", "tm44"]));
  const cells: ToggleCell[] = Array.from({ length: 12 }, (_, i) => {
    const n = 38 + i;
    return { id: `tm${n}`, label: `TM${n}`, name: machineNames[n], checked: on.has(`tm${n}`), disabled: n === 45 };
  });
  return (
    <>
      <ToggleGrid
        ariaLabel="TM und VM"
        cells={cells}
        columns={4}
        onToggle={(id, checked) =>
          setOn((s) => {
            const next = new Set(s);
            if (checked) next.add(id);
            else next.delete(id);
            return next;
          })
        }
      />
      <p className={styles.note}>
        {on.size} von {cells.length}
      </p>
    </>
  );
}

// Wanderer Jens's team: Staralili with a Trank, Bidiza; places 3 to 6 free.
const team = [
  { species: 396, name: "Staralili", level: 12, item: "Trank" },
  { species: 399, name: "Bidiza", level: 10 },
];

function TeamCards() {
  const [chosen, setChosen] = useState(0);
  return (
    <div className={styles.cardGrid}>
      {Array.from({ length: 6 }, (_, slot) => {
        const mon = team[slot];
        return mon ? (
          <PokemonCard
            key={slot}
            state={slot === chosen ? "selected" : "idle"}
            species={mon.species}
            name={mon.name}
            level={mon.level}
            item={mon.item}
            iconUrl={null}
            slot={slot + 1}
            ariaLabel={`Platz ${slot + 1}: ${mon.name}, Level ${mon.level}`}
            onClick={() => setChosen(slot)}
          />
        ) : (
          <PokemonCard key={slot} state="empty" slot={slot + 1} ariaLabel={`Platz ${slot + 1} leer`} onClick={() => {}} disabled={slot === 5} />
        );
      })}
    </div>
  );
}

export function DataPage() {
  const [types, setTypes] = useState<Set<string>>(new Set(["2"]));
  const [part, setPart] = useState("pokedex");
  const [highlight, setHighlight] = useState("price");

  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Prüfliste</h2>
        <p className={styles.note}>Jede Zeile sagt ihren Zustand mit Symbol und Wort, nie nur mit Farbe; rechts ein optionaler Zusatz.</p>
        <CheckList
          ariaLabel="Prüfliste"
          rows={[
            { id: "model", state: "ok", text: "Modell, 41 236 Bytes", trailing: <span className={styles.note}>passt</span> },
            { id: "names", state: "info", text: "3 Texturnamen fehlen im Gebiets-Paket" },
            { id: "bdhc", state: "warning", text: "BDHC bleibt unverändert, Laufhöhen prüfen" },
            { id: "per", state: "error", text: "Kollision: die Datei hat 2050 statt 2048 Bytes" },
          ]}
        />
      </section>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Typen</h2>
        <p className={styles.note}>Farbe und Name jedes Typs; eine Nummer, die die 4. Generation nicht kennt, heißt „Typ 19“.</p>
        <div className={styles.row}>
          {pokemonTypes.map((t) => (
            <TypeBadge key={t.id} type={t.id} />
          ))}
          <TypeBadge type={19} />
        </div>
        <div className={styles.row}>
          {pokemonTypes.map((t) => (
            <TypeBadge key={t.id} type={t.id} size="s" />
          ))}
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Basiswerte</h2>
        <p className={styles.note}>Strich: Schnitt aller Arten. Angriff weicht vom Original ab.</p>
        <div>
          {stats.map((s) => (
            <StatBar key={s.label} label={s.label} value={s.value} average={s.average} deviates={s.deviates} />
          ))}
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Filter</h2>
        <FilterChips ariaLabel="Typ" options={typeOptions} value={types} onChange={setTypes} />
        <FilterChips
          ariaLabel="Taschen"
          options={[
            { value: "items", label: "Gegenstände", count: 162 },
            { value: "medicine", label: "Medizin", count: 38 },
            { value: "balls", label: "Bälle", count: 15 },
          ]}
          value={new Set()}
          onChange={() => {}}
        />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Tabelle</h2>
        <p className={styles.note}>Tab landet auf einer Zeile, Pfeile wechseln die Zeile, Enter geht ins erste Feld, Escape zurück, Entf löscht die Zeile.</p>
        <div className={styles.panelBox}>
          <LearnsetTable />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Kästchen-Raster</h2>
        <p className={styles.note}>Pfeile in beiden Richtungen, Leertaste schaltet. Der Name der Attacke steht im zugänglichen Namen und im Tooltip.</p>
        <Machines />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Rohdaten</h2>
        <Segmented
          ariaLabel="Markiertes Feld"
          size="s"
          options={itemFields.map((f) => ({ value: f.name, label: itemLabels[f.name] ?? f.name }))}
          value={highlight}
          onChange={setHighlight}
        />
        <HexBlock bytes={itemBytes} fields={itemFields} highlight={highlight} path="itemtool/itemdata/pl_item_data.narc" member={17} labelOf={(n) => itemLabels[n] ?? n} />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Symbole</h2>
        <p className={styles.note}>Ohne Bild aus dem Projekt steht der Anfangsbuchstabe auf der Farbe des Bereichs.</p>
        <div className={styles.row}>
          <Sprite url={null} label="Staravia" size={24} area="pokemon" />
          <Sprite url={null} label="Staravia" size={32} area="pokemon" />
          <Sprite url={null} label="Staravia" size={48} area="pokemon" />
          <Sprite url={null} label="Trank" size={24} area="gegenstaende" />
          <Sprite url={null} label="Trank" size={32} area="gegenstaende" />
          <Sprite url={null} label="Trank" size={48} area="gegenstaende" />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Team-Karten</h2>
        <p className={styles.note}>Ein Platz im Team eines Trainers: gewählt mit Kobalt-Kante, frei gestrichelt, gesperrt blass. Klick, Enter oder Leertaste wählt.</p>
        <TeamCards />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Listenkopf mit Unterbereichen</h2>
        <div className={styles.startFrame}>
          <ListPane
            title="Pokémon"
            open
            onToggle={() => {}}
            switcher={{
              value: part,
              onChange: setPart,
              items: [
                { id: "pokedex", label: "Pokédex", count: 508, shortcut: "⌘1" },
                { id: "trainer", label: "Trainer", count: 928, shortcut: "⌘2" },
                { id: "encounters", label: "Begegnungen", disabled: true, hint: "kommt mit Phase 3b.3b", shortcut: "⌘3" },
                { id: "moves", label: "Attacken", count: 471, shortcut: "⌘4" },
              ],
            }}
          >
            <p className={styles.note}>Gewählt: {part}</p>
          </ListPane>
        </div>
      </section>
    </div>
  );
}
