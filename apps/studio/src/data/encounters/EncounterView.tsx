import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { errorText, isCmdError, splitEncounterFile } from "../../api";
import type { PlaceDto, SpeciesRowExtra } from "../../api";
import { Badge, Button, Dialog, Notice, Reference, Sprite, TypeBadge } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { useDataSnapshot } from "../dataStore";
import { monIcon, monIconKey, useIcon } from "../icons";
import { speciesName } from "../trainer/trainerModel";
import { useDelete, useEntityState } from "../../delete/DeleteContext";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { useDataArea } from "../useDataArea";
import { TIME_COLUMNS, partialSplit, shareText, warningText } from "./encounterModel";
import type { EncounterDoc, TimeKey } from "./encounterModel";
import { useEncounter } from "./useEncounter";
import styles from "./Encounter.module.css";

/** One species of „Wer hier wartet“: picture, name, types, and its share at each time of day („Tags 40 %“). */
function Waiting({ species, shares, types }: { species: number; shares: Readonly<Record<TimeKey, number>>; types: readonly number[] | null }) {
  const { names, goTo } = useDataArea("pokemon");
  const name = speciesName(names.species, species);
  const icon = useIcon(monIconKey(species), () => monIcon(species), true);
  const [a, b] = types ?? [];
  return (
    <li className={styles.waiting} data-species={species}>
      <Sprite url={icon} label={name} size={32} area="pokemon" decorative />
      <span className={styles.waitingName}>
        <Button kind="quiet" size="s" aria-label={`${name} im Pokédex öffnen`} onClick={() => goTo({ kind: "species", index: species })}>
          {name}
        </Button>
        {a !== undefined && (
          <span className={styles.badges}>
            <TypeBadge type={a} size="s" />
            {b !== undefined && b !== a && <TypeBadge type={b} size="s" />}
          </span>
        )}
      </span>
      <span className={styles.shares}>
        {TIME_COLUMNS.map((c) => (
          <span key={c.when} className={shares[c.when] === 0 ? styles.shareNone : styles.share}>{`${c.label} ${shareText(shares[c.when])}`}</span>
        ))}
      </span>
    </li>
  );
}

/** The species of the grass, each once, with its share at each time of day; largest first (by the most it ever has). */
function useWaiting(doc: EncounterDoc): { species: number; shares: Record<TimeKey, number> }[] {
  return useMemo(() => {
    const all = new Map<number, Record<TimeKey, number>>();
    for (const t of doc.effective) {
      for (const s of t.species) {
        const shares = all.get(s.species) ?? { morning: 0, day: 0, night: 0 };
        shares[t.when] = s.share;
        all.set(s.species, shares);
      }
    }
    const most = (s: Record<TimeKey, number>) => Math.max(s.morning, s.day, s.night);
    return [...all].map(([species, shares]) => ({ species, shares })).sort((x, y) => most(y.shares) - most(x.shares) || x.species - y.species);
  }, [doc.effective]);
}

/** The types of each species from the Pokédex list (read for this view). */
function useSpeciesTypes(): ReadonlyMap<number, readonly number[]> {
  const { project, store } = useDataArea("pokemon");
  const snapshot = useDataSnapshot(store);
  useEffect(() => {
    if (project !== null) store.need(project, "species");
  }, [project, store]);
  const rows = snapshot.project === project ? snapshot.rows.species : undefined;
  return useMemo(() => {
    const map = new Map<number, readonly number[]>();
    for (const r of rows ?? []) {
      const extra = r.extra as SpeciesRowExtra | null;
      if (extra && Array.isArray(extra.types)) map.set(r.index, extra.types);
    }
    return map;
  }, [rows]);
}

/** What a split that failed after writing its copy (code "partial") says, at file `at` (the copy) of the project at `root`. */
type SplitNote = { root: string; at: number; text: string };

/**
 * The note of the last partial split, kept outside the view: the view is not shown while the list is read again and the copy is selected.
 * The view shows it at its file and drops it once another file is chosen.
 */
const splitNotes = (() => {
  let note: SplitNote | null = null;
  const listeners = new Set<() => void>();
  return {
    get: (): SplitNote | null => note,
    set(next: SplitNote | null): void {
      note = next;
      for (const listener of [...listeners]) listener();
    },
    subscribe(listener: () => void): () => void {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
})();

/**
 * The question before a place gets a file of its own: the copy is written at once (not undoable), pointing the header at it is one undo
 * step. Afterwards the copy is selected. A split that wrote the copy but could not switch the header (code "partial") is caught up on like
 * one that went through, and the view says what stays (at the copy, as TrainerList does for a partial creation).
 */
function SplitDialog({ place, onClose }: { place: PlaceDto; onClose: () => void }) {
  const { bus, onEdited, select, project, selected } = useDataArea("pokemon");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  /** The list and the header table changed behind the editors (the files and their users with them); the copy is then selected. */
  const written = (copy: number | null, headers: boolean) => {
    if (headers) bus.emit({ topic: "headerTable" });
    bus.emit({ topic: "dataChanged", kind: "encounter", index: null });
    onEdited();
    onClose();
    if (copy !== null) select(copy);
  };
  const split = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const out = await splitEncounterFile(place.header);
      written(
        out.index,
        out.changed.some((a) => a.kind === "HeaderTable" || a.kind === "DynamicHeaders"),
      );
    } catch (e) {
      if (isCmdError(e) && e.code === "partial") {
        // The copy is on disk; whether the header changed is unknown, so the header table is read again too.
        const partial = partialSplit(e.message);
        const at = partial?.copy ?? selected;
        if (project !== null && at !== null) splitNotes.set({ root: project.root, at, text: partial?.text ?? e.message });
        written(partial?.copy ?? null, true);
      } else setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={`${place.name} eine eigene Begegnungsdatei geben?`}
      description={`Die Datei wird für ${place.name} kopiert, die anderen Orte behalten sie. Die Kopie wird sofort angelegt. Das Umhängen des Headers ist rückgängig.`}
      actions={[
        { label: "Abbrechen", onClick: onClose },
        { label: "Abspalten", kind: "primary", onClick: () => void split() },
      ]}
    >
      <div className={styles.dialogBody}>
        <span>
          <Badge kind="irreversible">Nicht rückgängig: die Kopie</Badge>
        </span>
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}

/** „Verwendet von N Orten“: each place as a reference into the world; with more than one, each can get a file of its own. */
export function Users({ doc }: { doc: EncounterDoc }) {
  const { goTo } = useDataArea("pokemon");
  const [splitting, setSplitting] = useState<PlaceDto | null>(null);
  const n = doc.users.length;
  return (
    <section className={styles.section} aria-label="Verwendet von">
      <h3 className={styles.sectionTitle}>{n === 0 ? "Von keinem Ort verwendet" : `Verwendet von ${n} ${n === 1 ? "Ort" : "Orten"}`}</h3>
      {n === 0 ? (
        <p className={styles.note}>Binde die Datei im Inspector eines Gebiets oder Raums an (Feld „Begegnungen“), dann gilt sie dort.</p>
      ) : (
        <ul className={styles.users}>
          {doc.users.map((u) => (
            <li key={u.header} className={styles.user}>
              <Reference area="welt" label={u.name} tech={`Header ${id4(u.header)}`} onOpen={() => goTo({ kind: "header", id: u.header })} dataAttrs={{ "data-header": u.header }} />
              {n > 1 && (
                <Button kind="quiet" size="s" aria-label={`${u.name}: für diesen Ort abspalten…`} disabled={doc.locked !== null} onClick={() => setSplitting(u)}>
                  Für diesen Ort abspalten
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      {splitting !== null && <SplitDialog place={splitting} onClose={() => setSplitting(null)} />}
    </section>
  );
}

/**
 * The view (Schau) of an encounter file: who waits in the grass with the share per time of day (from the kernel's sums), the places that
 * use the file (with „Für diesen Ort abspalten“ when several do), and the rules it breaks.
 */
export function EncounterView() {
  const { row, labelOf, selected, project } = useDataArea("pokemon");
  const { doc, error } = useEncounter(selected);
  const reset = useReset();
  const remove = useDelete();
  const state = useEntityState(selected === null ? null : { kind: "encounter", index: selected }, row);
  // A split that wrote its copy but failed later: said at the copy (or, without one named, at the file it was split from).
  const note = useSyncExternalStore(splitNotes.subscribe, splitNotes.get);
  const noted = note !== null && note.root === project?.root && note.at === selected ? note.text : null;
  useEffect(() => {
    const now = splitNotes.get();
    if (now !== null && (now.root !== project?.root || now.at !== selected)) splitNotes.set(null);
  }, [project, selected]);
  if (row === null || selected === null) return null;
  const title = labelOf(row);
  return (
    <section className={styles.view} aria-label={`Übersicht ${title}`} data-encounter={selected}>
      <div className={styles.viewBody}>
        <header className={styles.head}>
          <h2 className={styles.name}>{title}</h2>
          <span className={styles.tech}>{row.tech}</span>
          <HeadActions
            state={state}
            items={[{ label: "Löschen…", onSelect: () => remove.open({ kind: "encounter", index: selected }) }, resetEntry({ kind: "encounter", index: selected }, reset.status, reset.open)]}
          />
        </header>
        {noted !== null && <Notice kind="warning">{noted}</Notice>}
        {error !== null && <Notice kind="error">{`Diese Begegnungsdatei ist nicht lesbar: ${error}`}</Notice>}
        {doc && doc.locked !== null && <Notice kind="warning">{`${lockReason(doc.locked)}. Diese Datei ist gesperrt.`}</Notice>}
        {doc && doc.locked === null && <WhoWaits doc={doc} />}
        {doc && <Users doc={doc} />}
        {doc && <FileWarnings doc={doc} />}
        {doc === null && error === null && <p className={styles.note}>Lädt …</p>}
      </div>
    </section>
  );
}

/** The rules the file breaks, in German; nothing when it breaks none. */
export function FileWarnings({ doc }: { doc: EncounterDoc }) {
  const { names } = useDataArea("pokemon");
  if (doc.warnings.length === 0) return null;
  return (
    <section className={styles.section} aria-label="Warnungen">
      <h3 className={styles.sectionTitle}>
        Warnungen <span className={styles.count}>{doc.warnings.length}</span>
      </h3>
      {doc.warnings.map((w, i) => (
        <div key={i} data-warning={i}>
          <Notice kind="warning">{warningText(w.text, (id) => speciesName(names.species, id))}</Notice>
        </div>
      ))}
    </section>
  );
}

/** „Wer hier wartet“: the grass species with their share per time of day. */
function WhoWaits({ doc }: { doc: EncounterDoc }) {
  const waiting = useWaiting(doc);
  const types = useSpeciesTypes();
  const grass = doc.methods.find((m) => m.method === "grass");
  return (
    <section className={styles.section} aria-label="Wer hier wartet">
      <h3 className={styles.sectionTitle}>
        Wer hier wartet <span className={styles.count}>im Gras, Anteil je Tageszeit (Morgens 4–9, Tags 10–19, Nachts 20–3 Uhr)</span>
      </h3>
      {grass !== undefined && grass.rate === 0 && <p className={styles.note}>Gras-Rate 0: im Gras gibt es hier keine Begegnungen.</p>}
      {waiting.length === 0 ? (
        <p className={styles.note}>Im Gras wartet niemand.</p>
      ) : (
        <ul className={styles.waitingList}>
          {waiting.map((w) => (
            <Waiting key={w.species} species={w.species} shares={w.shares} types={types.get(w.species) ?? null} />
          ))}
        </ul>
      )}
    </section>
  );
}
