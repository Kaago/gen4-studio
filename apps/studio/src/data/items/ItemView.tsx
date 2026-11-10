import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { DataKind, DescriptionDto, NavTargetDto, RefDto } from "../../api";
import { Badge, Button, DSScreen, Notice, Reference, Sprite } from "../../ui/primitives";
import { id4, numberText } from "../../workbench/terms";
import { pages } from "../../workbench/text/textModel";
import { pocketLabel } from "../dataModel";
import { itemIcon, itemIconKey, useIcon } from "../icons";
import { useDataArea } from "../useDataArea";
import { useRefs } from "../useRefs";
import { dataMemberOf, hasIcon, itemUses, itemValues, lockText, sellPrice, sharedNote, usableInBattle, usesText } from "./itemModel";
import type { ItemValues } from "./itemModel";
import { useItem } from "./useItem";
import styles from "./Items.module.css";

/** The kinds whose change can add or drop a reference to an item: held items, evolutions by item, trainer items and teams. */
const REFERRING: readonly DataKind[] = ["species", "evolution", "trainer"];

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
            label={r.source_name}
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
  const { t } = useTranslation("data");
  const { goTo } = useDataArea("gegenstaende");
  const found = useRefs("gegenstaende", { kind: "item", id }, REFERRING);
  const uses = useMemo(() => (found?.refs ? itemUses(found.refs) : null), [found]);
  if (found === null) return <p className={styles.note}>{t("uses.searching")}</p>;
  if (uses === null) return <p className={styles.note}>{t("uses.unreadable", { error: found.error ?? "" })}</p>;
  const openSpecies = (nav: NavTargetDto) => {
    if (nav.kind === "species") goTo(nav.tab ? { kind: "species", index: nav.index, tab: nav.tab } : { kind: "species", index: nav.index });
  };
  const openTrainer = (nav: NavTargetDto) => {
    if (nav.kind === "trainer") goTo({ kind: "trainer", index: nav.index });
  };
  return (
    <>
      <h3 className={styles.sectionTitle}>
        {t("uses.usedIn")} <span className={styles.count}>{usesText(uses)}</span>
      </h3>
      <UseGroup title={t("itemView.groups.trainers")} refs={uses.trainers} open={openTrainer} />
      <UseGroup title={t("itemView.groups.parties")} refs={uses.parties} open={openTrainer} />
      <UseGroup title={t("itemView.groups.wild")} refs={uses.wild} open={openSpecies} />
      <UseGroup title={t("itemView.groups.evolutions")} refs={uses.evolutions} open={openSpecies} />
      <p className={styles.note}>{t("itemView.usesNote")}</p>
    </>
  );
}

/** The description as the game's text box shows it, page by page, with the way to its text. */
function Description({ description }: { description: DescriptionDto | null }) {
  const { t } = useTranslation("data");
  const { goTo } = useDataArea("gegenstaende");
  const all = useMemo(() => pages(description?.text ?? ""), [description]);
  const [page, setPage] = useState(0);
  const shown = Math.min(page, all.length - 1);
  return (
    <section className={styles.section} aria-label={t("description.title")}>
      <h3 className={styles.sectionTitle}>{t("description.title")}</h3>
      {description === null ? (
        <p className={styles.note}>{t("description.missing")}</p>
      ) : (
        <>
          <div className={styles.screen}>
            <DSScreen variant="textbox" pages={all} page={shown} scale={2} />
          </div>
          <div className={styles.actions}>
            {all.length > 1 && (
              <>
                <span className={styles.note}>{t("description.page", { page: shown + 1, pages: all.length })}</span>
                <Button kind="quiet" size="s" icon="back" disabled={shown === 0} onClick={() => setPage(shown - 1)}>
                  {t("description.back")}
                </Button>
                <Button kind="quiet" size="s" disabled={shown >= all.length - 1} onClick={() => setPage(shown + 1)}>
                  {t("description.nextPage")}
                </Button>
              </>
            )}
            <Reference
              area="texte"
              label={t("description.edit")}
              tech={t("archive", { id: id4(description.archive) })}
              onOpen={() => goTo({ kind: "textArchive", index: description.archive, message: description.message })}
            />
          </div>
        </>
      )}
    </section>
  );
}

/** The figures and badges of the card: pocket, use in battle (any battle pocket), price and selling price, fling power. */
function Facts({ v }: { v: ItemValues }) {
  const { t } = useTranslation("data");
  const price = v.price ?? 0;
  return (
    <>
      <div className={styles.badges}>
        <Badge kind="neutral">{pocketLabel(v.field_pocket ?? 0)}</Badge>
        {usableInBattle(v) && <Badge kind="neutral">{t("itemView.usableInBattle")}</Badge>}
        {(v.prevent_toss ?? 0) !== 0 && <Badge kind="neutral">{t("itemView.cannotToss")}</Badge>}
      </div>
      <div className={styles.figures}>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{`${price} ₽`}</span> {t("itemView.price")}
        </p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{`${sellPrice(price)} ₽`}</span> {t("itemView.sell")}
        </p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{v.fling_power ?? 0}</span> {t("itemView.flingPower")}
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
  const { t } = useTranslation("data");
  const { row, labelOf, selected, itemTable, goTo } = useDataArea("gegenstaende");
  const { doc, error } = useItem(selected, row === null ? null : dataMemberOf(row));
  const entry = selected === null ? undefined : itemTable?.[selected];
  const icon = row !== null && hasIcon(row, entry);
  const url = useIcon(icon ? itemIconKey(entry!) : null, () => (icon && selected !== null ? itemIcon(selected, entry!) : Promise.resolve(null)), true);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  const locked = doc?.locked ?? null;
  const shared = doc?.shared_with.length ?? 0;
  // The name in the item-name archive (message = item id); none without the item table.
  const nameArchive = doc?.name_archive ?? null;
  return (
    <section className={styles.view} aria-label={t("itemView.overview", { name })} data-item={selected}>
      <div className={styles.viewBody}>
        <section className={styles.card} aria-label={t("itemView.card")}>
          <header className={styles.cardHead}>
            <span className={styles.cardIcon}>
              <Sprite url={url} label={name} size={48} area="gegenstaende" />
            </span>
            <h2 className={styles.name}>
              {nameArchive === null ? name : <Reference area="texte" label={name} tech={t("archive", { id: id4(nameArchive) })} onOpen={() => goTo({ kind: "textArchive", index: nameArchive, message: selected })} />}
            </h2>
            <span className={styles.tech}>{numberText(row.index)}</span>
          </header>
          {error !== null && <Notice kind="error">{t("itemView.unreadable", { error })}</Notice>}
          {locked !== null && <Notice kind="warning">{t("itemView.lockedValues", { reason: lockText(locked) })}</Notice>}
          {doc !== null && locked === null && <Facts v={itemValues(doc.fields)} />}
          {shared > 0 && <p className={styles.note}>{sharedNote(shared)}</p>}
        </section>
        {doc !== null && <Description key={selected} description={doc.description} />}
        <section className={styles.section} aria-label={t("uses.usedIn")}>
          {itemTable ? <Uses id={selected} /> : <p className={styles.note}>{t("itemView.noTable")}</p>}
        </section>
      </div>
    </section>
  );
}
