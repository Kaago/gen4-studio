import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DataKind, DescriptionDto, NavTargetDto, RefDto } from "../../api";
import { errorText, refs } from "../../api";
import { Badge, Button, DSScreen, Notice, Reference, TypeBadge } from "../../ui/primitives";
import { pages } from "../../workbench/text/textModel";
import { useDataArea } from "../useDataArea";
import { NAMED_FLAGS, categoryName, hasFlag, id4, lockText, moveUses, moveValues, nameBank, priorityText, targetName, usesText } from "./moveModel";
import type { MoveValues } from "./moveModel";
import { useMove } from "./useMove";
import styles from "./Moves.module.css";

/** The kinds whose change can add or drop a reference to a move. */
const REFERRING: readonly DataKind[] = ["learnset", "trainer", "evolution"];

/** The references to move `index`, read again when a learnset, a trainer or an evolution changed. */
function useMoveRefs(index: number) {
  const { project, bus } = useDataArea("pokemon");
  const [found, setFound] = useState<{ index: number; refs: RefDto[] | null; error: string | null } | null>(null);
  const latest = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latest.current;
    try {
      const all = await refs({ kind: "move", index: i });
      if (latest.current === mine) setFound({ index: i, refs: all, error: null });
    } catch (e) {
      if (latest.current === mine) setFound({ index: i, refs: null, error: errorText(e) });
    }
  }, []);
  useEffect(() => {
    if (project === null) return;
    void read(index);
    return bus.subscribe("dataChanged", (event) => {
      if (event.topic === "dataChanged" && REFERRING.includes(event.kind)) void read(index);
    });
  }, [index, project, bus, read]);
  return found !== null && found.index === index ? found : null;
}

/** How many learnset chips show before "Alle zeigen". */
const FIRST_CHIPS = 12;

/** The species a reference names ("Staravia, Lernset Platz 4" → "Staravia"). */
const sourceName = (ref: RefDto) => ref.label.split(", ")[0] ?? ref.label;

/** "Verwendet in": the learnsets that teach the move, the evolutions it triggers, the trainers whose teams use it. */
function Uses({ index }: { index: number }) {
  const { goTo } = useDataArea("pokemon");
  const found = useMoveRefs(index);
  const [all, setAll] = useState(false);
  const uses = useMemo(() => (found?.refs ? moveUses(found.refs) : null), [found]);
  if (found === null) return <p className={styles.note}>Verwendungen werden gesucht …</p>;
  if (uses === null) return <p className={styles.note}>{`Verwendungen nicht lesbar: ${found.error ?? ""}`}</p>;
  const open = (nav: NavTargetDto) => {
    if (nav.kind === "species") goTo(nav.tab ? { kind: "species", index: nav.index, tab: nav.tab } : { kind: "species", index: nav.index });
  };
  const learnsets = all ? uses.learnsets : uses.learnsets.slice(0, FIRST_CHIPS);
  const more = uses.learnsets.length - learnsets.length;
  return (
    <section className={styles.section} aria-label="Verwendet in">
      <h3 className={styles.sectionTitle}>
        Verwendet in <span className={styles.count}>{usesText(uses)}</span>
      </h3>
      {uses.learnsets.length + uses.evolutions.length + uses.trainers.length > 0 && (
        <div className={styles.chips}>
          {learnsets.map((r) => (
            <Reference key={`l${r.source.index}`} area="pokemon" label={sourceName(r)} tech={id4(r.source.index)} onOpen={() => open(r.nav)} dataAttrs={{ "data-learnset": r.source.index }} />
          ))}
          {more > 0 && (
            <Button kind="quiet" size="s" onClick={() => setAll(true)}>
              {`Alle zeigen, ${more} weitere`}
            </Button>
          )}
          {uses.evolutions.map((r) => (
            <Reference key={`e${r.source.index}`} area="pokemon" label={r.label} onOpen={() => open(r.nav)} dataAttrs={{ "data-evolution": r.source.index }} />
          ))}
          {uses.trainers.map((r) => (
            <Reference key={`t${r.source.index}`} area="pokemon" label={sourceName(r)} tech={id4(r.source.index)} disabled onOpen={() => {}} dataAttrs={{ "data-trainer": r.source.index }} />
          ))}
        </div>
      )}
      {uses.trainers.length > 0 && <p className={styles.note}>Trainer öffnen kommt mit Phase 3b.3b.</p>}
      <p className={styles.note}>Lernsets und Trainer-Teams, ohne Scripts gezählt.</p>
    </section>
  );
}

/** The description as the game's text box shows it, page by page, with the way to its text. */
function Description({ description }: { description: DescriptionDto | null }) {
  const { goTo } = useDataArea("pokemon");
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

/** The figures and traits of the card. */
function Facts({ v }: { v: MoveValues }) {
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
        <p className={styles.figure}>{power === 0 ? "keine Stärke" : <><span className={styles.figureValue}>{power}</span> Stärke</>}</p>
        <p className={styles.figure}>{accuracy === 0 ? "trifft immer" : <><span className={styles.figureValue}>{`${accuracy} %`}</span> Genauigkeit</>}</p>
        <p className={styles.figure}>
          <span className={styles.figureValue}>{v.pp ?? 0}</span> AP
        </p>
      </div>
      <ul className={styles.traits} aria-label="Merkmale">
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
  const { row, labelOf, selected, project, goTo } = useDataArea("pokemon");
  const { doc, error } = useMove(selected);
  if (row === null || selected === null) return null;
  const name = labelOf(row);
  const bank = nameBank(project);
  const locked = doc?.locked ?? null;
  return (
    <section className={styles.view} aria-label={`Übersicht ${name}`} data-move={selected}>
      <div className={styles.viewBody}>
        <section className={styles.card} aria-label="Karte">
          <header className={styles.cardHead}>
            <h2 className={styles.name}>
              {bank === null ? name : <Reference area="texte" label={name} tech={`Archiv ${id4(bank)}`} onOpen={() => goTo({ kind: "textArchive", index: bank, message: selected })} />}
            </h2>
            <span className={styles.tech}>{`Nr. ${row.tech}`}</span>
          </header>
          {error !== null && <Notice kind="error">{`Diese Attacke ist nicht lesbar: ${error}`}</Notice>}
          {locked !== null && <Notice kind="warning">{`${lockText(locked, selected)}. Ihre Werte sind nicht lesbar.`}</Notice>}
          {doc !== null && locked === null && <Facts v={moveValues(doc.fields)} />}
        </section>
        {doc !== null && <Description key={selected} description={doc.description} />}
        <Uses index={selected} />
      </div>
    </section>
  );
}
