import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DataKind, DescriptionDto, NavTargetDto, RefDto } from "../../api";
import { errorText, refs } from "../../api";
import { Badge, Button, DSScreen, Notice, Reference, Sprite } from "../../ui/primitives";
import { pages } from "../../workbench/text/textModel";
import { pocketLabel } from "../dataModel";
import { itemIcon, itemIconKey, useIcon } from "../icons";
import { useDataArea } from "../useDataArea";
import { id4, itemUses, itemValues, lockText, sellPrice, sharedNote, usesText } from "./itemModel";
import type { ItemValues } from "./itemModel";
import { dataMemberOf, hasIcon } from "./ItemsList";
import { useItem } from "./useItem";
import styles from "./Items.module.css";

/** The kinds whose change can add or drop a reference to an item: held items, evolutions by item, trainer items and teams. */
const REFERRING: readonly DataKind[] = ["species", "evolution", "trainer"];

/** The references to item `id`, read again when a species, an evolution or a trainer changed. */
function useItemRefs(id: number) {
  const { project, bus } = useDataArea("gegenstaende");
  const [found, setFound] = useState<{ id: number; refs: RefDto[] | null; error: string | null } | null>(null);
  const latest = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latest.current;
    try {
      const all = await refs({ kind: "item", id: i });
      if (latest.current === mine) setFound({ id: i, refs: all, error: null });
    } catch (e) {
      if (latest.current === mine) setFound({ id: i, refs: null, error: errorText(e) });
    }
  }, []);
  useEffect(() => {
    if (project === null) return;
    void read(id);
    return bus.subscribe("dataChanged", (event) => {
      if (event.topic === "dataChanged" && REFERRING.includes(event.kind)) void read(id);
    });
  }, [id, project, bus, read]);
  return found !== null && found.id === id ? found : null;
}

/** The trainer or species a reference names ("Wanderer Kai, Gegenstand Platz 1" → "Wanderer Kai"). */
const sourceName = (ref: RefDto) => ref.label.split(", ")[0] ?? ref.label;

/** One group of "Verwendet in": its title with the count, and its chips. */
function UseGroup({ title, refs: list, open }: { title: string; refs: readonly RefDto[]; open: ((nav: NavTargetDto) => void) | null }) {
  if (list.length === 0) return null;
  return (
    <div className={styles.section} role="group" aria-label={title}>
      <p className={styles.note}>{`${title} ${list.length}`}</p>
      <div className={styles.chips}>
        {list.map((r) => (
          <Reference
            key={`${r.source.kind}${r.source.index}`}
            area="pokemon"
            label={sourceName(r)}
            tech={id4(r.source.index)}
            disabled={open === null}
            onOpen={() => open?.(r.nav)}
            dataAttrs={{ [`data-${r.source.kind === "Personal" ? "species" : r.source.kind === "Evolution" ? "evolution" : "trainer"}`]: r.source.index }}
          />
        ))}
      </div>
    </div>
  );
}

/** "Verwendet in": the trainers that carry the item, the teams that hold it, the species that hold it in the wild, the evolutions by it. */
function Uses({ id }: { id: number }) {
  const { goTo } = useDataArea("gegenstaende");
  const found = useItemRefs(id);
  const uses = useMemo(() => (found?.refs ? itemUses(found.refs) : null), [found]);
  if (found === null) return <p className={styles.note}>Verwendungen werden gesucht …</p>;
  if (uses === null) return <p className={styles.note}>{`Verwendungen nicht lesbar: ${found.error ?? ""}`}</p>;
  const openSpecies = (nav: NavTargetDto) => {
    if (nav.kind === "species") goTo(nav.tab ? { kind: "species", index: nav.index, tab: nav.tab } : { kind: "species", index: nav.index });
  };
  return (
    <>
      <h3 className={styles.sectionTitle}>
        Verwendet in <span className={styles.count}>{usesText(uses)}</span>
      </h3>
      <UseGroup title="Trainer-Items" refs={uses.trainers} open={null} />
      <UseGroup title="Team-Items" refs={uses.parties} open={null} />
      <UseGroup title="Wild getragen" refs={uses.wild} open={openSpecies} />
      <UseGroup title="Entwicklungen" refs={uses.evolutions} open={openSpecies} />
      {uses.trainers.length + uses.parties.length > 0 && <p className={styles.note}>Trainer öffnen kommt mit Phase 3b.3b.</p>}
      <p className={styles.note}>Trainer, Teams und Pokémon-Daten, ohne Scripts und Läden gezählt.</p>
    </>
  );
}

/** The description as the game's text box shows it, page by page, with the way to its text. */
function Description({ description }: { description: DescriptionDto | null }) {
  const { goTo } = useDataArea("gegenstaende");
  const all = useMemo(() => pages(description?.text ?? ""), [description]);
  const [page, setPage] = useState(0);
  const shown = Math.min(page, all.length - 1);
  return (
    <section className={styles.section} aria-label="Beschreibung">
      <h3 className={styles.sectionTitle}>Beschreibung</h3>
      {description === null ? (
        <p className={styles.note}>Beschreibung nicht gefunden.</p>
      ) : (
        <>
          <div className={styles.screen}>
            <DSScreen variant="textbox" pages={all} page={shown} scale={2} />
          </div>
          <div className={styles.actions}>
            {all.length > 1 && (
              <>
                <span className={styles.note}>{`Seite ${shown + 1} von ${all.length}`}</span>
                <Button kind="quiet" size="s" icon="back" disabled={shown === 0} onClick={() => setPage(shown - 1)}>
                  Zurück
                </Button>
                <Button kind="quiet" size="s" disabled={shown >= all.length - 1} onClick={() => setPage(shown + 1)}>
                  Weiterblättern
                </Button>
              </>
            )}
            <Reference
              area="texte"
              label="Beschreibung bearbeiten"
              tech={`Archiv ${id4(description.archive)}`}
              onOpen={() => goTo({ kind: "textArchive", index: description.archive, message: description.message })}
            />
          </div>
        </>
      )}
    </section>
  );
}

/** The figures and badges of the card: pocket, use in battle, price and selling price, fling power. */
function Facts({ v }: { v: ItemValues }) {
  const price = v.price ?? 0;
  return (
    <>
      <div className={styles.badges}>
        <Badge kind="neutral">{pocketLabel(v.field_pocket ?? 0)}</Badge>
        {(v.battle_use_func ?? 0) !== 0 && <Badge kind="neutral">Im Kampf nutzbar</Badge>}
        {(v.prevent_toss ?? 0) !== 0 && <Badge kind="neutral">Nicht wegwerfbar</Badge>}
      </div>
      <div className={styles.figures}>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{`${price} ₽`}</span> Preis
        </p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{`${sellPrice(price)} ₽`}</span> Verkauf
        </p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{v.fling_power ?? 0}</span> Schleuder-Stärke
        </p>
      </div>
    </>
  );
}

/**
 * The view (Schau) of an item: its card (icon from the ROM, pocket, prices, the items sharing its data), its description on the DS screen,
 * where it is used.
 */
export function ItemView() {
  const { row, labelOf, selected, itemTable } = useDataArea("gegenstaende");
  const { doc, error } = useItem(selected, row === null ? null : dataMemberOf(row));
  const entry = selected === null ? undefined : itemTable?.[selected];
  const icon = row !== null && hasIcon(row, entry);
  const url = useIcon(icon ? itemIconKey(entry!) : null, () => (icon && selected !== null ? itemIcon(selected, entry!) : Promise.resolve(null)), true);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  const locked = doc?.locked ?? null;
  const shared = doc?.shared_with.length ?? 0;
  return (
    <section className={styles.view} aria-label={`Übersicht ${name}`} data-item={selected}>
      <div className={styles.viewBody}>
        <section className={styles.card} aria-label="Karte">
          <header className={styles.cardHead}>
            <span className={styles.cardIcon}>
              <Sprite url={url} label={name} size={48} area="gegenstaende" />
            </span>
            <h2 className={styles.name}>{name}</h2>
            <span className={styles.tech}>{`Nr. ${row.tech}`}</span>
          </header>
          {error !== null && <Notice kind="error">{`Dieser Gegenstand ist nicht lesbar: ${error}`}</Notice>}
          {locked !== null && <Notice kind="warning">{`${lockText(locked, selected)}. Seine Werte sind nicht lesbar.`}</Notice>}
          {doc !== null && locked === null && <Facts v={itemValues(doc.fields)} />}
          {shared > 0 && <p className={styles.note}>{sharedNote(shared)}</p>}
        </section>
        {doc !== null && <Description key={selected} description={doc.description} />}
        <section className={styles.section} aria-label="Verwendet in">
          {itemTable ? <Uses id={selected} /> : <p className={styles.note}>Ohne Item-Tabelle lassen sich die Verwendungen nicht suchen.</p>}
        </section>
      </div>
    </section>
  );
}
