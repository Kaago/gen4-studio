import { useId, useRef, useState } from "react";
import type { ReactNode, Ref } from "react";
import type { Against, BlobSide, ChangedEntry, EntityDiff as Diff, EntityDiffReport, FieldChange, OriginalStatus, WordSpan } from "../api";
import { useReset } from "../original/OriginalContext";
import { RESET_LABEL } from "../original/resetEntry";
import { Badge, Button, ChangeTable, Notice, Section, Tooltip } from "../ui/primitives";
import type { ChangeCell, ChangeTableRow } from "../ui/primitives";
import { headerFieldSpecs } from "../workbench/inspector/fields";
import type { NavTarget } from "../workbench/navigation";
import { navTargetOf, rowResetLock, sideWords } from "./diffWords";
import styles from "./EntityDiff.module.css";
import { useEntityDiff } from "./useEntityDiff";

/** Why „Öffnen“ is locked. */
const NO_PLACE = "Für diese Art gibt es keinen Ort in der App.";

/** The German names of a map's parts, in the kernel's order. */
const PART_NAMES: Record<string, string> = { model: "Modell", permissions: "Bewegungsrechte", buildings: "Gebäude", bdhc: "BDHC", bgs: "BGS", raw: "Rohdaten" };

export type EntityDiffProps = {
  /** The row chosen in the list; null: nothing chosen yet. */
  entry: ChangedEntry | null;
  against: Against;
  /** Moves whenever the compared state may differ: the diff is read again. */
  revision: number;
  /** The id of `HEAD` (null without a commit or while unknown): against the last commit the detail reads again when it moves. */
  headId?: string | null;
  /** „Öffnen“: the place of the entity in the app. */
  goTo: (target: NavTarget) => void;
  /** Where focus goes after a reset from here once the detail's own button is gone (the row became equal and the choice went with it). */
  finalFocus?: () => HTMLElement | null;
};

/**
 * Why reset is locked: as on the entity's row (its state, its kind, the base ROM: read, missing, or without the entry), and „Kein Original“
 * for an entity that is new on the compared side.
 */
function resetLock(entry: ChangedEntry, report: EntityDiffReport | null, status: OriginalStatus | null): string | null {
  if (report?.state === "new") return "Kein Original";
  if (report !== null && report.reset === null) return "Für diese Art gibt es kein Zurücksetzen.";
  return rowResetLock(report?.reset ? { ...entry, target: report.reset } : entry, status);
}

/**
 * The detail of Technik › Änderungen: what of the chosen entity differs from the compared side, in the shape of its kind (fields, a map's
 * parts and cells, a text archive's messages with word spans, a script's lines, a binary part's size and SHA-1), with „Öffnen“ and
 * „Auf Original zurücksetzen…“. Without a choice it says what to do.
 */
export function EntityDiff({ entry, against, revision, headId = null, goTo, finalFocus }: EntityDiffProps) {
  const { report, error, busy } = useEntityDiff(entry?.subject ?? null, against, revision, headId);
  if (entry === null) {
    return (
      <div className={styles.detail} data-entity-diff="">
        <p className={styles.note}>Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.</p>
      </div>
    );
  }
  return (
    <div className={styles.detail} data-entity-diff="">
      <Head entry={entry} report={report} goTo={goTo} finalFocus={finalFocus} />
      {report === null && busy && (
        <p role="status" className={styles.note}>
          Lade Vergleich …
        </p>
      )}
      {error !== null && <Notice kind="error">{error}</Notice>}
      {report !== null && (report.state === "same" ? <p className={styles.text}>Keine Abweichung.</p> : <Body report={report} against={against} />)}
    </div>
  );
}

/**
 * A secondary button that may be locked: then focusable all the same, its reason the description and the tooltip (as on the rows).
 * `waiting` locks it without a reason (what it does is not known yet).
 */
function LockableButton({ why, waiting = false, onClick, children, ref }: { why: string | null; waiting?: boolean; onClick: () => void; children: ReactNode; ref?: Ref<HTMLButtonElement> }) {
  const noteId = useId();
  const off = waiting || why !== null;
  const button = (
    <Button ref={ref} kind="secondary" size="s" aria-disabled={off || undefined} aria-describedby={why !== null ? noteId : undefined} onClick={() => !off && onClick()}>
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

function Head({ entry, report, goTo, finalFocus }: { entry: ChangedEntry; report: EntityDiffReport | null; goTo: (t: NavTarget) => void; finalFocus?: () => HTMLElement | null }) {
  const { open, status } = useReset();
  const resetButton = useRef<HTMLButtonElement>(null);
  const place = navTargetOf(entry.subject, report?.diff ?? null);
  // An event file's place follows from its diff: until that is read, „Öffnen“ waits without a reason.
  const waiting = report === null && entry.subject.kind === "event_file";
  const target = report?.reset ?? entry.target;
  const locked = resetLock(entry, report, status);
  return (
    <header className={styles.head}>
      <div className={styles.headText}>
        <h2 className={styles.title}>{report?.label ?? entry.label}</h2>
        {report !== null && (
          <p className={styles.sides}>
            {report.old_label} → {report.new_label}
          </p>
        )}
      </div>
      <div className={styles.actions}>
        <LockableButton why={place === null && !waiting ? NO_PLACE : null} waiting={waiting} onClick={() => place !== null && goTo(place)}>
          Öffnen
        </LockableButton>
        <LockableButton
          ref={resetButton}
          why={locked}
          onClick={() =>
            target !== null &&
            open(target, { finalFocus: () => (resetButton.current?.isConnected ? resetButton.current : (finalFocus?.() ?? null)) })
          }
        >
          {RESET_LABEL}
        </LockableButton>
      </div>
    </header>
  );
}

/** Spans as a cell; none (a side without the message) as missing. */
const spans = (list: readonly WordSpan[], absent: boolean): ChangeCell => (absent ? null : list);

/** The rows of field changes; with `specs` (a header's) a field by its German name, the path as tooltip. */
function fieldRows(changes: readonly FieldChange[], specs: readonly { key: string; label: string }[] = []): ChangeTableRow[] {
  return changes.map((c) => {
    const name = specs.find((s) => s.key === c.path)?.label;
    return { key: c.path, field: name ?? c.path, ...(name !== undefined && { title: c.path }), old: c.old, new: c.new };
  });
}

/** „und {n} weitere …“ below a table, when rows were cut. */
const moreText = (more: number, what: string) => (more > 0 ? `und ${more} weitere ${what}` : undefined);

function Body({ report, against }: { report: EntityDiffReport; against: Against }) {
  const words = sideWords(against);
  const family = useReset().family;
  const diff = report.diff;
  switch (diff.shape) {
    case "fields":
      return (
        <ChangeTable
          columns={["Feld", words.old, words.new]}
          ariaLabel={report.label}
          rows={fieldRows(diff.changes, report.subject.kind === "header" && family ? headerFieldSpecs(family) : [])}
          more={moreText(diff.more, "Felder")}
        />
      );
    case "map":
      return <MapDiff diff={diff} label={report.label} words={words} />;
    case "messages":
      return (
        <ChangeTable
          columns={["Nachricht", words.old, words.new]}
          ariaLabel={report.label}
          rows={diff.messages.map((m) => ({
            key: String(m.index),
            field: (
              <span className={styles.message}>
                {m.index}
                {m.state === "new" && <Badge kind="info">Neu</Badge>}
                {m.state === "removed" && <Badge kind="error">Entfernt</Badge>}
              </span>
            ),
            old: spans(m.old, m.state === "new"),
            new: spans(m.new, m.state === "removed"),
          }))}
          more={moreText(diff.more, "Nachrichten")}
        />
      );
    case "lines":
      return <LineDiff diff={diff} />;
    case "binary":
      return <ChangeTable columns={["Eigenschaft", words.old, words.new]} ariaLabel={report.label} rows={binaryRows(diff.old, diff.new)} />;
  }
}

/** Size and SHA-1 of a binary side; „wie in der Basis-ROM“ where the text project keeps none, missing where the side has none. */
function blobCell(side: BlobSide, what: "size" | "sha1"): ChangeCell {
  if (side.side === "absent") return null;
  if (side.side === "base") return "wie in der Basis-ROM";
  return what === "size" ? `${side.size.toLocaleString("de-DE")} Byte` : side.sha1.slice(0, 8);
}

const binaryRows = (old: BlobSide, now: BlobSide): ChangeTableRow[] => [
  { key: "size", field: "Größe", old: blobCell(old, "size"), new: blobCell(now, "size") },
  { key: "sha1", field: "SHA-1", old: blobCell(old, "sha1"), new: blobCell(now, "sha1") },
];

function MapDiff({ diff, label, words }: { diff: Extract<Diff, { shape: "map" }>; label: string; words: { old: string; new: string } }) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const toggle = (name: string) =>
    setFolded((before) => {
      const next = new Set(before);
      if (!next.delete(name)) next.add(name);
      return next;
    });
  return (
    <>
      <div className={styles.sections}>
        <Section title="Teile" open={!folded.has("parts")} onToggle={() => toggle("parts")}>
          <ul className={styles.parts} data-map-parts="">
            {diff.parts.map((p) => (
              <li key={p.part} className={styles.part}>
                <span>{PART_NAMES[p.part] ?? p.part}</span>
                {p.changed ? <Badge kind="warning">geändert</Badge> : <Badge kind="neutral">gleich</Badge>}
              </li>
            ))}
          </ul>
        </Section>
        {diff.cells.length > 0 && (
          <Section title="Zellen der Bewegungsrechte" count={diff.cells.length} open={!folded.has("cells")} onToggle={() => toggle("cells")}>
            <ChangeTable
              columns={["Zelle", words.old, words.new]}
              ariaLabel={`${label}: Zellen der Bewegungsrechte`}
              // A side without such a cell (the map grew or shrank) is missing.
              rows={diff.cells.map((c) => ({ key: `${c.x},${c.y}`, field: `${c.x}, ${c.y}`, old: c.old === "" ? null : c.old, new: c.new === "" ? null : c.new }))}
            />
          </Section>
        )}
        {diff.buildings.length > 0 && (
          <Section title="Gebäude" count={diff.buildings.length} open={!folded.has("buildings")} onToggle={() => toggle("buildings")}>
            <ChangeTable columns={["Feld", words.old, words.new]} ariaLabel={`${label}: Gebäude`} rows={fieldRows(diff.buildings)} />
          </Section>
        )}
      </div>
      {/* The kernel cuts cells and building fields together: one count below both. */}
      {diff.more > 0 && <p className={styles.more}>{`und ${diff.more} weitere Einträge`}</p>}
    </>
  );
}

/** The hunks of a line diff: where each starts on both sides, then its lines with „−“, „+“ or a space in front. */
function LineDiff({ diff }: { diff: Extract<Diff, { shape: "lines" }> }) {
  return (
    <div className={styles.hunks}>
      {diff.hunks.map((hunk) => (
        <div key={`${hunk.old_start}:${hunk.new_start}`} className={styles.hunk} data-hunk="">
          <h3 className={styles.hunkTitle}>
            Zeile {hunk.old_start} → {hunk.new_start}
          </h3>
          <code className={styles.lines}>
            {hunk.lines.map(([op, text], i) =>
              op === "removed" ? (
                <del key={i} className={`${styles.line} ${styles.removed}`} data-line="">
                  {`−${text}`}
                </del>
              ) : op === "added" ? (
                <ins key={i} className={`${styles.line} ${styles.added}`} data-line="">
                  {`+${text}`}
                </ins>
              ) : (
                <span key={i} className={styles.line} data-line="">
                  {` ${text}`}
                </span>
              ),
            )}
          </code>
        </div>
      ))}
      {diff.more > 0 && <p className={styles.note}>{`und ${diff.more} weitere Zeilen`}</p>}
    </div>
  );
}
