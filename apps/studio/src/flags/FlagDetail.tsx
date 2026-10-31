import { useId, useState } from "react";
import type { ReactNode } from "react";
import type { FlagEntryDto, FlagKind, FlagRefDto, FlagUsageDto } from "../api";
import { useEntityIds } from "../textproject/useEntityIds";
import type { Depth } from "../ui/frame/shellState";
import { Button, Notice, Reference, Section, Tooltip } from "../ui/primitives";
import type { NavTarget } from "../workbench/navigation";
import { groupRefsByHeader } from "./flagFilter";
import { NAMING_NEEDS_TEXT_PROJECT, OUTSIDE_THE_SAVE, areaWord, displayName, numberText } from "./flagWords";
import styles from "./FlagsSurface.module.css";

/** Why naming waits while a save or a build runs (the palette's sentence). */
const BUSY = "Erst Speichern oder Bauen abwarten";

/** What a number does when only read: the hint in „Wo gesetzt“. */
export const READ_NOT_SET = "Gelesen, aber nirgends gesetzt: die Engine oder ein Script über eine Variable setzt es, oder nie.";

export type FlagDetailProps = {
  kind: FlagKind;
  /** The chosen number; null: nothing chosen. */
  number: number | null;
  /** The number's row of the list (names, area) while its usage is read; null when the list does not hold it. */
  entry: FlagEntryDto | null;
  usage: FlagUsageDto | null;
  busy: boolean;
  error: string | null;
  /** The project is a text project: own names can be given (else the DSPRE layout locks naming with the way there). */
  textProject: boolean;
  /** The project takes no writes now (a save or a build runs). */
  readOnly: boolean;
  /** Tüfteln heads the map-local groups with the header's id. */
  depth: Depth;
  goTo: (target: NavTarget) => void;
  /** „Umbenennen…“ of an own name. */
  onRename: (id: string) => void;
  /** „Als eigenes Flag benennen…“ / „Als eigene Variable benennen…“. */
  onName: () => void;
  /** „Beschreibung…“ of an own name. */
  onDescribe: () => void;
};

/** A secondary button that may be locked: focusable all the same, its reason the description and the tooltip. */
function HeadButton({ why, onClick, children }: { why: string | null; onClick: () => void; children: ReactNode }) {
  const noteId = useId();
  const button = (
    <Button kind="secondary" size="s" aria-disabled={why !== null || undefined} aria-describedby={why !== null ? noteId : undefined} onClick={() => why === null && onClick()}>
      {children}
    </Button>
  );
  if (why === null) return button;
  return (
    <>
      <Tooltip content={why}>{button}</Tooltip>
      <span id={noteId} hidden>
        {why}
      </span>
    </>
  );
}

type Group = { key: string; name: string; refs: FlagRefDto[] };

/** The references grouped by source („Scriptdatei 0012“), in the order of their first one. */
function bySource(refs: readonly FlagRefDto[]): Group[] {
  const groups = new Map<string, Group>();
  for (const ref of refs) {
    let group = groups.get(ref.source_name);
    if (group === undefined) groups.set(ref.source_name, (group = { key: ref.source_name, name: ref.source_name, refs: [] }));
    group.refs.push(ref);
  }
  return [...groups.values()];
}

/** One section of the detail: its groups, each a heading and the chips of its references; `empty` when it has none. */
function Uses({ title, groups, empty, goTo }: { title: string; groups: Group[]; empty: ReactNode; goTo: (target: NavTarget) => void }) {
  const [open, setOpen] = useState(true);
  const count = groups.reduce((n, g) => n + g.refs.length, 0);
  return (
    <div data-flag-section={title}>
      <Section title={title} count={count} open={open} onToggle={() => setOpen((o) => !o)}>
        {count === 0 ? (
          empty
        ) : (
          <div className={styles.groups}>
            {groups.map((g) => (
              <div key={g.key} className={styles.group} data-flag-group={g.name}>
                <h4 className={styles.groupHead}>{g.name}</h4>
                <div className={styles.uses}>
                  {g.refs.map((r, i) => (
                    <Reference
                      key={i}
                      area={r.nav.kind === "trainer" ? "pokemon" : "welt"}
                      label={r.label}
                      tech={r.source_name}
                      onOpen={() => goTo(r.nav)}
                      dataAttrs={{ "data-flag-use": r.access }}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </Section>
    </div>
  );
}

/**
 * The chosen number: its name (an own one over the pret name), number and area (and the recipe that reserves it), its description, the
 * naming („Umbenennen…“ and „Beschreibung…“ with an own name, else „Als eigenes Flag benennen…“; locked with the reason in the DSPRE layout
 * and past the save), then „Wo gesetzt“, „Wo geprüft“ and „Unklar“ (only when there is something unclear) with a chip per reference that
 * follows its way. A map-local number groups by header (in Tüfteln by the header's id), every other by source.
 */
export function FlagDetail({ kind, number, entry, usage, busy, error, textProject, readOnly, depth, goTo, onRename, onName, onDescribe }: FlagDetailProps) {
  const names = usage ?? entry;
  const area = names?.area ?? "general";
  const mapLocal = area === "map_local";
  const ids = useEntityIds("Map", number !== null && textProject && mapLocal && depth === "tuefteln");
  if (number === null) {
    return (
      <div className={styles.detail} data-flag-detail="">
        <p className={styles.empty}>Wähle links ein Flag oder eine Variable.</p>
      </div>
    );
  }
  const own = names?.own_id ?? null;
  const pret = names?.name ?? null;
  const reserved = names?.reserved ?? null;
  const description = names?.description ?? null;
  const why = !textProject ? NAMING_NEEDS_TEXT_PROJECT : area === "outside" ? OUTSIDE_THE_SAVE : readOnly ? BUSY : null;
  const refs = usage?.refs ?? [];
  const groups = (list: FlagRefDto[]): Group[] =>
    mapLocal
      ? groupRefsByHeader(list).map((g) => ({ key: String(g.header), name: (g.header !== null ? ids?.[g.header] : undefined) ?? g.name, refs: g.refs }))
      : bySource(list);
  const set = refs.filter((r) => r.access === "write" || r.access === "read_write");
  const checked = refs.filter((r) => r.access === "read" || r.access === "read_write");
  const unclear = refs.filter((r) => r.access === "unknown");
  return (
    <div className={styles.detail} data-flag-detail="">
      <header className={styles.detailHead}>
        <h2 className={own !== null ? `${styles.detailTitle} ${styles.own}` : styles.detailTitle}>{displayName({ name: pret, own_id: own }, kind, number)}</h2>
        {own !== null && pret !== null && <p className={styles.pret}>{pret}</p>}
        <p className={styles.facts}>{`${numberText(number)} · ${areaWord(area)}${reserved !== null ? ` · ${reserved}` : ""}`}</p>
        <p className={description !== null ? styles.text : styles.note}>{description ?? "Keine Beschreibung"}</p>
        <div className={styles.actions}>
          {own !== null ? (
            <>
              <HeadButton why={why} onClick={() => onRename(own)}>
                Umbenennen…
              </HeadButton>
              <HeadButton why={why} onClick={onDescribe}>
                Beschreibung…
              </HeadButton>
            </>
          ) : (
            <HeadButton why={why} onClick={onName}>
              {kind === "flag" ? "Als eigenes Flag benennen…" : "Als eigene Variable benennen…"}
            </HeadButton>
          )}
        </div>
      </header>
      {error !== null && <Notice kind="error">{error}</Notice>}
      {usage === null && busy && error === null && <p className={styles.note}>Wird gelesen …</p>}
      {usage !== null && (
        <>
          <Uses
            title="Wo gesetzt"
            groups={groups(set)}
            empty={checked.length > 0 ? <Notice kind="info">{READ_NOT_SET}</Notice> : <p className={styles.note}>Nirgends gesetzt.</p>}
            goTo={goTo}
          />
          <Uses title="Wo geprüft" groups={groups(checked)} empty={<p className={styles.note}>Nirgends geprüft.</p>} goTo={goTo} />
          {unclear.length > 0 && <Uses title="Unklar" groups={groups(unclear)} empty={null} goTo={goTo} />}
        </>
      )}
    </div>
  );
}
