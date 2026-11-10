import { useId, useRef, useState } from "react";
import type { ReactNode, Ref } from "react";
import { useTranslation } from "react-i18next";
import type { Against, BlobSide, ChangedEntry, EntityDiff as Diff, EntityDiffReport, FieldChange, OriginalStatus, WordSpan } from "../api";
import { restoreLabel, useDelete } from "../delete/DeleteContext";
import { t } from "../i18n";
import { useReset } from "../original/OriginalContext";
import { resetLabel } from "../original/resetEntry";
import { Badge, Button, ChangeTable, Notice, Section, Tooltip } from "../ui/primitives";
import type { ChangeCell, ChangeTableRow } from "../ui/primitives";
import { headerFieldSpecs } from "../workbench/inspector/fields";
import type { NavTarget } from "../workbench/navigation";
import { navTargetOf, restoreTarget, rowResetLock, sideWords } from "./diffWords";
import styles from "./EntityDiff.module.css";
import { useEntityDiff } from "./useEntityDiff";

/** The parts of a map the catalog has a name for, in the kernel's order. */
const PART_NAMES = ["model", "permissions", "buildings", "bdhc", "bgs", "raw"] as const;
type PartName = (typeof PART_NAMES)[number];
const isPartName = (part: string): part is PartName => (PART_NAMES as readonly string[]).includes(part);
/** The name of a map's part in the language that is shown; a part the catalog does not know keeps the kernel's name. */
const partName = (part: string): string => (isPartName(part) ? t(`technik:entityDiff.part.${part}`) : part);

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
  if (report?.state === "new") return t("technik:diffWords.noOriginal");
  if (report !== null && report.reset === null) return t("technik:diffWords.noReset");
  return rowResetLock(report?.reset ? { ...entry, target: report.reset } : entry, status);
}

/**
 * The detail of Technik › Änderungen: what of the chosen entity differs from the compared side, in the shape of its kind (fields, a map's
 * parts and cells, a text archive's messages with word spans, a script's lines, a binary part's size and SHA-1), with „Öffnen“ and
 * „Auf Original zurücksetzen…“ („Wiederherstellen“ for a new entity deleted). A tombstone that differs between the sides (3d.4) is the badge
 * „Gelöscht“ (or „Wiederhergestellt“ when only the old side is one) above the fields, never a field. Without a choice it says what to do.
 */
export function EntityDiff({ entry, against, revision, headId = null, goTo, finalFocus }: EntityDiffProps) {
  const { t } = useTranslation("technik");
  const { report, error, busy } = useEntityDiff(entry?.subject ?? null, against, revision, headId);
  if (entry === null) {
    return (
      <div className={styles.detail} data-entity-diff="">
        <p className={styles.note}>{t("entityDiff.choose")}</p>
      </div>
    );
  }
  return (
    <div className={styles.detail} data-entity-diff="">
      <Head entry={entry} report={report} goTo={goTo} finalFocus={finalFocus} />
      {report === null && busy && (
        <p role="status" className={styles.note}>
          {t("entityDiff.loading")}
        </p>
      )}
      {error !== null && <Notice kind="error">{error}</Notice>}
      {report !== null &&
        (report.state === "same" ? (
          <p className={styles.text}>{t("entityDiff.same")}</p>
        ) : (
          <>
            <Tombstone report={report} />
            {!(tombstoneDiffers(report) && nothingElse(report.diff)) && <Body report={report} against={against} />}
          </>
        ))}
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

/** Whether one side is a tombstone and the other is not. */
const tombstoneDiffers = (report: EntityDiffReport) => report.tombstone !== null && report.tombstone.old !== report.tombstone.new;

/** Whether the diff names nothing but the envelope: no field, no changed part, cell or building. */
function nothingElse(diff: Diff): boolean {
  if (diff.shape === "fields") return diff.changes.length === 0 && diff.more === 0;
  if (diff.shape === "map") return !diff.parts.some((p) => p.changed) && diff.cells.length === 0 && diff.buildings.length === 0 && diff.more === 0;
  return false;
}

/** The tombstone of the new side as the badge „Gelöscht“, of the old side only as „Wiederhergestellt“; nothing when the sides agree. */
function Tombstone({ report }: { report: EntityDiffReport }) {
  const { t } = useTranslation("technik");
  if (!tombstoneDiffers(report)) return null;
  const deleted = report.tombstone!.new;
  return (
    <p className={styles.tombstone} data-tombstone={deleted ? "deleted" : "restored"}>
      {deleted ? <Badge kind="error">{t("entityDiff.deleted")}</Badge> : <Badge kind="info">{t("entityDiff.restored")}</Badge>}
    </p>
  );
}

function Head({ entry, report, goTo, finalFocus }: { entry: ChangedEntry; report: EntityDiffReport | null; goTo: (t: NavTarget) => void; finalFocus?: () => HTMLElement | null }) {
  const { t } = useTranslation("technik");
  const { open, status } = useReset();
  const { restore } = useDelete();
  const back = restoreTarget(entry);
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
        <LockableButton why={place === null && !waiting ? t("entityDiff.noPlace") : null} waiting={waiting} onClick={() => place !== null && goTo(place)}>
          {t("shared.open")}
        </LockableButton>
        {back !== null ? (
          <LockableButton why={null} onClick={() => restore(back)}>
            {restoreLabel()}
          </LockableButton>
        ) : (
          <LockableButton
            ref={resetButton}
            why={locked}
            onClick={() =>
              target !== null &&
              open(target, { finalFocus: () => (resetButton.current?.isConnected ? resetButton.current : (finalFocus?.() ?? null)) })
            }
          >
            {resetLabel()}
          </LockableButton>
        )}
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
const moreText = (more: number, what: "Fields" | "Messages") => (more > 0 ? t(`technik:entityDiff.more${what}`, { count: more }) : undefined);

function Body({ report, against }: { report: EntityDiffReport; against: Against }) {
  const { t } = useTranslation("technik");
  const words = sideWords(against);
  const family = useReset().family;
  const diff = report.diff;
  switch (diff.shape) {
    case "fields":
      return (
        <ChangeTable
          columns={[t("entityDiff.columns.field"), words.old, words.new]}
          ariaLabel={report.label}
          rows={fieldRows(diff.changes, report.subject.kind === "header" && family ? headerFieldSpecs(family) : [])}
          more={moreText(diff.more, "Fields")}
        />
      );
    case "map":
      return <MapDiff diff={diff} label={report.label} words={words} />;
    case "messages":
      return (
        <ChangeTable
          columns={[t("entityDiff.columns.message"), words.old, words.new]}
          ariaLabel={report.label}
          rows={diff.messages.map((m) => ({
            key: String(m.index),
            field: (
              <span className={styles.message}>
                {m.index}
                {m.state === "new" && <Badge kind="info">{t("entityDiff.messageNew")}</Badge>}
                {m.state === "removed" && <Badge kind="error">{t("entityDiff.messageRemoved")}</Badge>}
              </span>
            ),
            old: spans(m.old, m.state === "new"),
            new: spans(m.new, m.state === "removed"),
          }))}
          more={moreText(diff.more, "Messages")}
        />
      );
    case "lines":
      return <LineDiff diff={diff} />;
    case "binary":
      return <ChangeTable columns={[t("entityDiff.columns.property"), words.old, words.new]} ariaLabel={report.label} rows={binaryRows(diff.old, diff.new)} />;
  }
}

/** Size and SHA-1 of a binary side; „wie in der Basis-ROM“ where the text project keeps none, missing where the side has none. */
function blobCell(side: BlobSide, what: "size" | "sha1"): ChangeCell {
  if (side.side === "absent") return null;
  if (side.side === "base") return t("technik:entityDiff.asInBase");
  return what === "size" ? t("technik:entityDiff.bytes", { count: side.size }) : side.sha1.slice(0, 8);
}

const binaryRows = (old: BlobSide, now: BlobSide): ChangeTableRow[] => [
  { key: "size", field: t("technik:shared.size"), old: blobCell(old, "size"), new: blobCell(now, "size") },
  { key: "sha1", field: "SHA-1", old: blobCell(old, "sha1"), new: blobCell(now, "sha1") },
];

function MapDiff({ diff, label, words }: { diff: Extract<Diff, { shape: "map" }>; label: string; words: { old: string; new: string } }) {
  const { t } = useTranslation("technik");
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
        <Section title={t("entityDiff.partsTitle")} open={!folded.has("parts")} onToggle={() => toggle("parts")}>
          <ul className={styles.parts} data-map-parts="">
            {diff.parts.map((p) => (
              <li key={p.part} className={styles.part}>
                <span>{partName(p.part)}</span>
                {p.changed ? <Badge kind="warning">{t("entityDiff.partChanged")}</Badge> : <Badge kind="neutral">{t("entityDiff.partSame")}</Badge>}
              </li>
            ))}
          </ul>
        </Section>
        {diff.cells.length > 0 && (
          <Section title={t("entityDiff.cellsTitle")} count={diff.cells.length} open={!folded.has("cells")} onToggle={() => toggle("cells")}>
            <ChangeTable
              columns={[t("entityDiff.columns.cell"), words.old, words.new]}
              ariaLabel={t("entityDiff.cellsAriaLabel", { label })}
              // A side without such a cell (the map grew or shrank) is missing.
              rows={diff.cells.map((c) => ({ key: `${c.x},${c.y}`, field: `${c.x}, ${c.y}`, old: c.old === "" ? null : c.old, new: c.new === "" ? null : c.new }))}
            />
          </Section>
        )}
        {diff.buildings.length > 0 && (
          <Section title={t("entityDiff.buildingsTitle")} count={diff.buildings.length} open={!folded.has("buildings")} onToggle={() => toggle("buildings")}>
            <ChangeTable columns={[t("entityDiff.columns.field"), words.old, words.new]} ariaLabel={t("entityDiff.buildingsAriaLabel", { label })} rows={fieldRows(diff.buildings)} />
          </Section>
        )}
      </div>
      {/* The kernel cuts cells and building fields together: one count below both. */}
      {diff.more > 0 && <p className={styles.more}>{t("entityDiff.moreEntries", { count: diff.more })}</p>}
    </>
  );
}

/** The hunks of a line diff: where each starts on both sides, then its lines with „−“, „+“ or a space in front. */
function LineDiff({ diff }: { diff: Extract<Diff, { shape: "lines" }> }) {
  const { t } = useTranslation("technik");
  return (
    <div className={styles.hunks}>
      {diff.hunks.map((hunk) => (
        <div key={`${hunk.old_start}:${hunk.new_start}`} className={styles.hunk} data-hunk="">
          <h3 className={styles.hunkTitle}>
            {t("entityDiff.hunkTitle", { old: hunk.old_start, new: hunk.new_start })}
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
      {diff.more > 0 && <p className={styles.note}>{t("entityDiff.moreLines", { count: diff.more })}</p>}
    </div>
  );
}
