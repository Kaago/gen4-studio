import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { DataKind, DescriptionDto, NavTargetDto } from "../../api";
import { Badge, Button, DSScreen, Notice, Reference, TypeBadge } from "../../ui/primitives";
import { id4, numberText } from "../../workbench/terms";
import { pages } from "../../workbench/text/textModel";
import { useDataArea } from "../useDataArea";
import { useRefs } from "../useRefs";
import { NAMED_FLAGS, categoryName, hasFlag, lockText, moveUses, moveValues, priorityText, targetName, usesText } from "./moveModel";
import type { MoveValues } from "./moveModel";
import { useMove } from "./useMove";
import styles from "./Moves.module.css";

/** The kinds whose change can add or drop a reference to a move. */
const REFERRING: readonly DataKind[] = ["learnset", "trainer", "evolution"];

/** How many learnset chips show before "Alle zeigen". */
const FIRST_CHIPS = 12;

/** "Verwendet in": the learnsets that teach the move, the evolutions it triggers, the trainers whose teams use it. */
function Uses({ index }: { index: number }) {
  const { t } = useTranslation("data");
  const { goTo } = useDataArea("pokemon");
  const found = useRefs("pokemon", { kind: "move", index }, REFERRING);
  const [all, setAll] = useState(false);
  const uses = useMemo(() => (found?.refs ? moveUses(found.refs) : null), [found]);
  if (found === null) return <p className={styles.note}>{t("uses.searching")}</p>;
  if (uses === null) return <p className={styles.note}>{t("uses.unreadable", { error: found.error ?? "" })}</p>;
  const open = (nav: NavTargetDto) => {
    if (nav.kind === "species") goTo(nav.tab ? { kind: "species", index: nav.index, tab: nav.tab } : { kind: "species", index: nav.index });
    else if (nav.kind === "trainer") goTo({ kind: "trainer", index: nav.index });
  };
  const learnsets = all ? uses.learnsets : uses.learnsets.slice(0, FIRST_CHIPS);
  const more = uses.learnsets.length - learnsets.length;
  return (
    <section className={styles.section} aria-label={t("uses.usedIn")}>
      <h3 className={styles.sectionTitle}>
        {t("uses.usedIn")} <span className={styles.count}>{usesText(uses)}</span>
      </h3>
      {uses.learnsets.length + uses.evolutions.length + uses.trainers.length > 0 && (
        <div className={styles.chips}>
          {learnsets.map((r) => (
            <Reference key={`l${r.source.index}`} area="pokemon" label={r.source_name} tech={id4(r.source.index)} onOpen={() => open(r.nav)} dataAttrs={{ "data-learnset": r.source.index }} />
          ))}
          {more > 0 && (
            <Button kind="quiet" size="s" onClick={() => setAll(true)}>
              {t("moveView.showAll", { more })}
            </Button>
          )}
          {uses.evolutions.map((r) => (
            <Reference key={`e${r.source.index}`} area="pokemon" label={r.label} onOpen={() => open(r.nav)} dataAttrs={{ "data-evolution": r.source.index }} />
          ))}
          {uses.trainers.map((r) => (
            <Reference key={`t${r.source.index}`} area="pokemon" label={r.source_name} tech={id4(r.source.index)} onOpen={() => open(r.nav)} dataAttrs={{ "data-trainer": r.source.index }} />
          ))}
        </div>
      )}
      <p className={styles.note}>{t("moveView.usesNote")}</p>
    </section>
  );
}

/** The description as the game's text box shows it, page by page, with the way to its text. */
function Description({ description }: { description: DescriptionDto | null }) {
  const { t } = useTranslation("data");
  const { goTo } = useDataArea("pokemon");
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

/** The figures and traits of the card. */
function Facts({ v }: { v: MoveValues }) {
  const { t } = useTranslation("data");
  const power = v.power ?? 0;
  const accuracy = v.accuracy ?? 0;
  const flags = v.flags ?? 0;
  return (
    <>
      <div className={styles.badges}>
        <TypeBadge type={v.move_type ?? 0} />
        <Badge kind="neutral">{categoryName(v.category ?? 0)}</Badge>
      </div>
      <div className={styles.figures}>
        <p className={styles.figure}>{power === 0 ? t("moveView.noPower") : <><span className={styles.figureValue}>{power}</span> {t("moveView.power")}</>}</p>
        <p className={styles.figure}>{accuracy === 0 ? t("moveView.alwaysHits") : <><span className={styles.figureValue}>{`${accuracy} %`}</span> {t("moveView.accuracy")}</>}</p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{v.pp ?? 0}</span> {t("moveView.pp")}
        </p>
      </div>
      <ul className={styles.traits} aria-label={t("moveView.traits")}>
        {NAMED_FLAGS.filter((f) => hasFlag(flags, f.mask)).map((f) => (
          <li key={f.bit}>
            <Badge kind="neutral" icon="check">
              {f.label}
            </Badge>
          </li>
        ))}
        <li>
          <Badge kind="neutral">{priorityText(v.priority ?? 0)}</Badge>
        </li>
        <li>
          <Badge kind="neutral">{targetName(v.target ?? 0)}</Badge>
        </li>
      </ul>
    </>
  );
}

/** The view (Schau) of a move: its card (type, category, power, accuracy, PP, traits), its description on the DS screen, where it is used. */
export function MoveView() {
  const { t } = useTranslation("data");
  const { row, labelOf, selected, goTo } = useDataArea("pokemon");
  const { doc, error } = useMove(selected);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  // The text archive of the move names, as the backend names it for this game and language.
  const bank = doc?.name_archive ?? null;
  const locked = doc?.locked ?? null;
  return (
    <section className={styles.view} aria-label={t("moveView.overview", { name })} data-move={selected}>
      <div className={styles.viewBody}>
        <section className={styles.card} aria-label={t("moveView.card")}>
          <header className={styles.cardHead}>
            <h2 className={styles.name}>
              {bank === null ? name : <Reference area="texte" label={name} tech={t("archive", { id: id4(bank) })} onOpen={() => goTo({ kind: "textArchive", index: bank, message: selected })} />}
            </h2>
            <span className={styles.tech}>{numberText(row.index)}</span>
          </header>
          {error !== null && <Notice kind="error">{t("moveView.unreadable", { error })}</Notice>}
          {locked !== null && <Notice kind="warning">{t("moveView.lockedValues", { reason: lockText(locked, selected) })}</Notice>}
          {doc !== null && locked === null && <Facts v={moveValues(doc.fields)} />}
        </section>
        {doc !== null && <Description key={`description${selected}`} description={doc.description} />}
        <Uses key={`uses${selected}`} index={selected} />
      </div>
    </section>
  );
}
