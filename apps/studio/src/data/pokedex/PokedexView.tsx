import { id4, termLabel } from "../../workbench/terms";
import { useMemo, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { BattleSpriteRequest, DataKind, HeaderSummary, NavTargetDto, RefDto, SpeciesRowExtra, SpriteSlotsDto } from "../../api";
import { DSPokedexProfile, Notice, Reference, Segmented, Sprite, StatBar, Switch, TypeBadge } from "../../ui/primitives";
import type { Area } from "../../ui/primitives";
import type { NavTarget } from "../../workbench/navigation";
import { battleSprite, battleSpriteKey, monIcon, monIconKey, useIcon } from "../icons";
import { HeadActions } from "../../original/HeadActions";
import { EntityIdLine } from "../../textproject/EntityIdLine";
import { useTextProject } from "../../textproject/TextProjectContext";
import { useEntityIds } from "../../textproject/useEntityIds";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { useDataArea } from "../useDataArea";
import { useRefs } from "../useRefs";
import { STATS, abilityName, lockText, lockedParts, statAverages, statSum } from "./pokedexModel";
import { useSpecies } from "./useSpecies";
import styles from "./Pokedex.module.css";

/** The kinds whose change can add or drop a reference to a species. */
const REFERRING: readonly DataKind[] = ["encounter", "trainer", "evolution"];

/** Where a reference leads in the app (an encounter file a header uses leads to that place; one nobody uses to the Begegnungen). */
function navTarget(nav: NavTargetDto): NavTarget | null {
  switch (nav.kind) {
    case "trainer":
      return { kind: "trainer", index: nav.index };
    case "encounter":
      return { kind: "encounter", index: nav.index };
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
function placeName(ref: RefDto, headers: readonly HeaderSummary[], t: TFunction<"data">): string {
  if (ref.nav.kind !== "header") return `${t("dataModel.fallbackWord.encounter")} ${id4(ref.source.index)}`;
  const id = ref.nav.id;
  const header = headers.find((h) => h.id === id);
  return header?.location_name || header?.internal_name || `${termLabel("header")} ${id4(id)}`;
}

/** "Fundorte" (encounters and trainer parties) and "Verwendet in" (everything else that names the species). */
function Uses({ index }: { index: number }) {
  const { t } = useTranslation("data");
  const { headers, goTo } = useDataArea("pokemon");
  const found = useRefs("pokemon", { kind: "species", index }, REFERRING);
  if (found === null) return <p className={styles.note}>{t("pokedexView.searching")}</p>;
  if (found.error !== null || found.refs === null) return <p className={styles.note}>{t("pokedexView.placesUnreadable", { error: found.error ?? "" })}</p>;
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
      <section className={styles.section} aria-label={t("pokedexView.places")}>
        <h3 className={styles.sectionTitle}>
          {t("pokedexView.places")} <span className={styles.count}>{t("pokedexView.placesCount", { places: t("pokedexView.placeCount", { count: places.length }), trainers: t("pokedexView.trainerParties", { count: trainers.length }) })}</span>
        </h3>
        {places.length + trainers.length === 0 ? (
          <p className={styles.note}>{t("pokedexView.nowhere")}</p>
        ) : (
          <div className={styles.chips}>
            {places.map((r) => (
              <Reference
                key={`p${r.source.index}`}
                area="welt"
                label={placeName(r, headers, t)}
                tech={id4(r.source.index)}
                disabled={navTarget(r.nav) === null}
                onOpen={() => open(r.nav)}
                dataAttrs={{ "data-place": r.source.index }}
              />
            ))}
            {trainers.map((r) => (
              <Reference key={`t${r.source.index}`} area="pokemon" label={r.source_name} tech={id4(r.source.index)} onOpen={() => open(r.nav)} dataAttrs={{ "data-trainer": r.source.index }} />
            ))}
          </div>
        )}
      </section>
      <section className={styles.section} aria-label={t("uses.usedIn")}>
        <h3 className={styles.sectionTitle}>{t("uses.usedIn")}</h3>
        {other.length > 0 && (
          <div className={styles.chips}>
            {other.map((r, i) => (
              <Reference key={i} area={areaOf(r.nav)} label={r.label} disabled={navTarget(r.nav) === null} onOpen={() => open(r.nav)} dataAttrs={{ "data-use": i }} />
            ))}
          </div>
        )}
        <p className={styles.note}>{other.length === 0 ? t("pokedexView.otherNone") : t("pokedexView.otherUses", { count: other.length })}</p>
      </section>
    </>
  );
}

/**
 * "Spielerblick": the Pokédex page as the player sees it on the DS (number, name, types, the battle picture), with the side, ♀ and shiny
 * switches. The switches belong to the view for the session (nothing is saved; `PokedexView` holds them, so the loading gap between two species does not reset them). A gender without an image of its own is locked: the
 * backend answers the other gender's image silently, so the lock tells what the picture shows.
 */
type PlayerChoice = { side: "front" | "back"; female: boolean; shiny: boolean };

function PlayerView({
  index,
  number,
  name,
  types,
  slots,
  choice: { side, female, shiny },
  onChoice,
}: {
  index: number;
  number: number;
  name: string;
  types: readonly [number] | readonly [number, number];
  slots: SpriteSlotsDto | null;
  choice: PlayerChoice;
  onChoice: (patch: Partial<PlayerChoice>) => void;
}) {
  // Unknown slots (no readable sprites) lock nothing: the picture is the placeholder anyway.
  const { t } = useTranslation("data");
  const sides = [
    { value: "front", label: t("pokedexView.front") },
    { value: "back", label: t("pokedexView.back") },
  ] as const;
  const noFemale = slots !== null && !slots.female;
  const onlyFemale = slots !== null && !slots.male && slots.female;
  const isFemale = onlyFemale || (!noFemale && female);
  const request: BattleSpriteRequest = { species: index, side, female: isFemale, shiny };
  const url = useIcon(battleSpriteKey(request), () => battleSprite(request), true);
  const lockText = noFemale ? t("pokedexView.noFemale") : onlyFemale ? t("pokedexView.onlyFemale") : null;
  return (
    <section className={styles.section} aria-label={t("pokedexView.playerView")} data-ds-profile-section="">
      <h3 className={styles.sectionTitle}>{t("pokedexView.playerView")}</h3>
      <DSPokedexProfile number={number} name={name} types={types} spriteUrl={url} decorative />
      <div className={styles.profileControls}>
        <div data-side={side}>
          <Segmented options={sides} value={side} onChange={(next) => onChoice({ side: next })} ariaLabel={t("pokedexView.side")} size="s" />
        </div>
        <div data-female={isFemale}>
          <Switch label={t("pokedexView.female")} checked={isFemale} onChange={(next) => onChoice({ female: next })} disabled={lockText !== null} hint={lockText ?? undefined} />
        </div>
        <div data-shiny={shiny}>
          <Switch label={t("pokedexView.shiny")} checked={shiny} onChange={(next) => onChoice({ shiny: next })} />
        </div>
      </div>
    </section>
  );
}

/** The view (Schau) of a species: name, types and abilities, its base stats against the average of all species, where it is found and used. */
export function PokedexView() {
  const { t } = useTranslation("data");
  const { row, rows, labelOf, names, selected } = useDataArea("pokemon");
  const { doc, error } = useSpecies(selected);
  const reset = useReset();
  const averages = useMemo(() => statAverages(rows), [rows]);
  const [choice, setChoice] = useState<PlayerChoice>({ side: "front", female: false, shiny: false });
  const icon = useIcon(selected === null ? null : monIconKey(selected), () => monIcon(selected ?? 0), true);
  // In a text project the head names the species' id (a fixed one: it is not renamed).
  const ids = useEntityIds("Species", useTextProject().active);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  const entityId = ids?.[selected] ?? null;
  const personalLocked = doc !== null && lockedParts(doc).personal;
  const personal = doc?.personal;
  const abilities = personal ? [personal.ability1, personal.ability2].filter((a, i, all) => a !== 0 && all.indexOf(a) === i) : [];
  return (
    <section className={styles.view} aria-label={t("pokedexView.overview", { name })}>
      <header className={styles.head}>
        <Sprite url={icon} label={name} size={48} area="pokemon" decorative />
        <div className={styles.headText}>
          <h2 className={styles.name}>{name}</h2>
          <span className={styles.tech}>{row.tech}</span>
          {entityId !== null && <EntityIdLine id={entityId} />}
        </div>
        {personal && !personalLocked && (
          <div className={styles.headFacts}>
            <span className={styles.badges}>
              <TypeBadge type={personal.type1} />
              {personal.type2 !== personal.type1 && <TypeBadge type={personal.type2} />}
            </span>
            <span className={styles.abilities}>
              {abilities.length === 0 ? t("pokedexModel.noAbility") : t("pokedexView.abilityList", { count: abilities.length, list: abilities.map((a) => abilityName(names.abilities, a)).join(", ") })}
            </span>
          </div>
        )}
        {/* The species' data, learnset and evolution, in one step. */}
        <HeadActions items={[resetEntry({ kind: "species", index: selected }, reset.status, reset.open)]} />
      </header>
      <div className={styles.viewBody}>
        {error !== null && <Notice kind="error">{t("pokedexView.unreadable", { error })}</Notice>}
        {doc !== null && personalLocked && <Notice kind="warning">{t("pokedexView.lockedValues", { reason: lockText(doc.locked ?? "", doc.locked_asset) })}</Notice>}
        {doc !== null && !personalLocked && (
          <PlayerView
            index={selected}
            number={(row.extra as SpeciesRowExtra | null)?.number ?? selected}
            name={name}
            types={[doc.personal.type1, doc.personal.type2]}
            slots={doc.sprite_slots}
            choice={choice}
            onChoice={(patch) => setChoice((c) => ({ ...c, ...patch }))}
          />
        )}
        {doc !== null && !personalLocked && (
          <section className={styles.section} aria-label={t("pokedexView.baseStats")}>
            <h3 className={styles.sectionTitle}>{t("pokedexView.baseStats")}</h3>
            <div>
              {STATS.map((s) => (
                <StatBar key={s.field} label={s.label} value={doc.personal[s.field]} average={averages[s.field]} deviates={doc.personal[s.field] !== doc.saved[s.field]} />
              ))}
            </div>
            <p className={styles.sum}>{sumText(t, statSum(doc.personal), statSum(doc.saved))}</p>
            <p className={styles.note}>{t("pokedexView.baseStatsNote")}</p>
          </section>
        )}
        <Uses index={selected} />
      </div>
    </section>
  );
}

const sumText = (t: TFunction<"data">, now: number, saved: number) => (now === saved ? t("pokedexView.sum", { now }) : t("pokedexView.sumChanged", { now, saved }));
