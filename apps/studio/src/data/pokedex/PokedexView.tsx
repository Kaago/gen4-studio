import { id4 } from "../../workbench/terms";
import { useMemo } from "react";
import type { DataKind, HeaderSummary, NavTargetDto, RefDto } from "../../api";
import { Notice, Reference, Sprite, StatBar, TypeBadge } from "../../ui/primitives";
import type { Area } from "../../ui/primitives";
import type { NavTarget } from "../../workbench/navigation";
import { monIcon, monIconKey, useIcon } from "../icons";
import { useDataArea } from "../useDataArea";
import { useRefs } from "../useRefs";
import { STATS, abilityName, lockText, lockedParts, statAverages, statSum } from "./pokedexModel";
import { useSpecies } from "./useSpecies";
import styles from "./Pokedex.module.css";

/** The kinds whose change can add or drop a reference to a species. */
const REFERRING: readonly DataKind[] = ["encounter", "trainer", "evolution"];

/** Where a reference leads in the app; null for a target without an editor yet (encounters and trainers come with 3b.3b). */
function navTarget(nav: NavTargetDto): NavTarget | null {
  switch (nav.kind) {
    case "header":
      return { kind: "header", id: nav.id };
    case "event":
      return { kind: "event", eventFile: nav.eventFile, eventKind: nav.eventKind, index: nav.index };
    case "species":
      return nav.tab ? { kind: "species", index: nav.index, tab: nav.tab } : { kind: "species", index: nav.index };
    default:
      return null;
  }
}

/** The area whose colour a reference takes. */
const areaOf = (nav: NavTargetDto): Area => (nav.kind === "species" || nav.kind === "trainer" ? "pokemon" : "welt");

/** One reference per member: the first of each source member. */
function firstPer(list: readonly RefDto[]): RefDto[] {
  const seen = new Set<number>();
  return list.filter((r) => {
    if (seen.has(r.source.index)) return false;
    seen.add(r.source.index);
    return true;
  });
}

/** The place of an encounter file: the location of the header that uses it, else the file. */
function placeName(ref: RefDto, headers: readonly HeaderSummary[]): string {
  if (ref.nav.kind !== "header") return `Begegnungen ${id4(ref.source.index)}`;
  const id = ref.nav.id;
  const header = headers.find((h) => h.id === id);
  return header?.location_name || header?.internal_name || `Header ${id4(id)}`;
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** "Fundorte" (encounters and trainer parties) and "Verwendet in" (everything else that names the species). */
function Uses({ index }: { index: number }) {
  const { headers, goTo } = useDataArea("pokemon");
  const found = useRefs("pokemon", { kind: "species", index }, REFERRING);
  if (found === null) return <p className={styles.note}>Fundorte werden gesucht …</p>;
  if (found.error !== null || found.refs === null) return <p className={styles.note}>{`Fundorte nicht lesbar: ${found.error ?? ""}`}</p>;
  const all = found.refs;
  const places = firstPer(all.filter((r) => r.source.kind === "Encounter"));
  const trainers = firstPer(all.filter((r) => r.source.kind === "TrainerParty" || r.source.kind === "TrainerProperties"));
  const other = all.filter((r) => r.source.kind !== "Encounter" && r.source.kind !== "TrainerParty" && r.source.kind !== "TrainerProperties");
  const open = (nav: NavTargetDto) => {
    const target = navTarget(nav);
    if (target !== null) goTo(target);
  };
  return (
    <>
      <section className={styles.section} aria-label="Fundorte">
        <h3 className={styles.sectionTitle}>
          Fundorte <span className={styles.count}>{`${plural(places.length, "Ort", "Orte")}, ${plural(trainers.length, "Trainer-Team", "Trainer-Teams")}`}</span>
        </h3>
        {places.length + trainers.length === 0 ? (
          <p className={styles.note}>Wartet in keiner Begegnung und in keinem Trainer-Team.</p>
        ) : (
          <div className={styles.chips}>
            {places.map((r) => (
              <Reference
                key={`p${r.source.index}`}
                area="welt"
                label={placeName(r, headers)}
                tech={id4(r.source.index)}
                disabled={navTarget(r.nav) === null}
                onOpen={() => open(r.nav)}
                dataAttrs={{ "data-place": r.source.index }}
              />
            ))}
            {trainers.map((r) => (
              <Reference key={`t${r.source.index}`} area="pokemon" label={r.source_name} tech={id4(r.source.index)} disabled onOpen={() => {}} dataAttrs={{ "data-trainer": r.source.index }} />
            ))}
          </div>
        )}
        {trainers.length > 0 && <p className={styles.note}>Trainer öffnen kommt mit Phase 3b.3b.</p>}
      </section>
      <section className={styles.section} aria-label="Verwendet in">
        <h3 className={styles.sectionTitle}>Verwendet in</h3>
        {other.length > 0 && (
          <div className={styles.chips}>
            {other.map((r, i) => (
              <Reference key={i} area={areaOf(r.nav)} label={r.label} disabled={navTarget(r.nav) === null} onOpen={() => open(r.nav)} dataAttrs={{ "data-use": i }} />
            ))}
          </div>
        )}
        <p className={styles.note}>{`${other.length === 0 ? "Nirgends sonst" : plural(other.length, "Stelle", "Stellen")}, ohne Scripts gezählt.`}</p>
      </section>
    </>
  );
}

/** The view (Schau) of a species: name, types and abilities, its base stats against the average of all species, where it is found and used. */
export function PokedexView() {
  const { row, rows, labelOf, names, selected } = useDataArea("pokemon");
  const { doc, error } = useSpecies(selected);
  const averages = useMemo(() => statAverages(rows), [rows]);
  const icon = useIcon(selected === null ? null : monIconKey(selected), () => monIcon(selected ?? 0), true);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  const personalLocked = doc !== null && lockedParts(doc).personal;
  const personal = doc?.personal;
  const abilities = personal ? [personal.ability1, personal.ability2].filter((a, i, all) => a !== 0 && all.indexOf(a) === i) : [];
  return (
    <section className={styles.view} aria-label={`Übersicht ${name}`}>
      <header className={styles.head}>
        <Sprite url={icon} label={name} size={48} area="pokemon" decorative />
        <div className={styles.headText}>
          <h2 className={styles.name}>{name}</h2>
          <span className={styles.tech}>{row.tech}</span>
        </div>
        {personal && !personalLocked && (
          <div className={styles.headFacts}>
            <span className={styles.badges}>
              <TypeBadge type={personal.type1} />
              {personal.type2 !== personal.type1 && <TypeBadge type={personal.type2} />}
            </span>
            <span className={styles.abilities}>
              {abilities.length === 0 ? "Keine Fähigkeit" : `${abilities.length === 1 ? "Fähigkeit" : "Fähigkeiten"}: ${abilities.map((a) => abilityName(names.abilities, a)).join(", ")}`}
            </span>
          </div>
        )}
      </header>
      <div className={styles.viewBody}>
        {error !== null && <Notice kind="error">{`Diese Art ist nicht lesbar: ${error}`}</Notice>}
        {doc !== null && personalLocked && <Notice kind="warning">{`${lockText(doc.locked ?? "", doc.locked_asset)}. Ihre Werte sind nicht lesbar.`}</Notice>}
        {doc !== null && !personalLocked && (
          <section className={styles.section} aria-label="Basiswerte">
            <h3 className={styles.sectionTitle}>Basiswerte</h3>
            <div>
              {STATS.map((s) => (
                <StatBar key={s.field} label={s.label} value={doc.personal[s.field]} average={averages[s.field]} deviates={doc.personal[s.field] !== doc.saved[s.field]} />
              ))}
            </div>
            <p className={styles.sum}>{sumText(statSum(doc.personal), statSum(doc.saved))}</p>
            <p className={styles.note}>Strich: Schnitt aller Arten. Vorher: Stand der letzten Speicherung.</p>
          </section>
        )}
        <Uses index={selected} />
      </div>
    </section>
  );
}

const sumText = (now: number, saved: number) => (now === saved ? `Summe ${now}` : `Summe ${now}, vorher ${saved}`);
