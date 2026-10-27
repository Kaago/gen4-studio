import { useRef, useState } from "react";
import type { TextKeep } from "../api";
import { Dialog, Segmented } from "../ui/primitives";
import type { SegmentedOption } from "../ui/primitives";
import styles from "./TextConflictDialog.module.css";

/** One archive's choice: which version applies (the other is overwritten). */
export type TextConflictChoice = { archive: number; keep: TextKeep };

export type TextConflictDialogProps = {
  /** The text archives in conflict, in the order the build named them. */
  archives: number[];
  /** „Übernehmen und bauen“: the choice per archive, in the same order. */
  onResolve: (choices: TextConflictChoice[]) => void;
  /** „Abbrechen“, Escape, the close button or a click outside: nothing is built. */
  onCancel: () => void;
};

const OPTIONS: readonly SegmentedOption<TextKeep>[] = [
  { value: "json", label: "Text (JSON)" },
  { value: "member", label: "Spieldatei" },
];

const archiveName = (archive: number) => `Textarchiv ${String(archive).padStart(4, "0")}`;

/**
 * The question a build puts when text archives changed as JSON and as member since the studio last built them (spec 4.2 „Konflikt beim
 * Bauen“): per archive which version applies, the JSON by default. Nothing is overwritten silently: „Abbrechen“ builds nothing. Answered
 * once: the Escape of the closing dialog after „Übernehmen und bauen“ cancels nothing.
 */
export function TextConflictDialog({ archives, onResolve, onCancel }: TextConflictDialogProps) {
  const [keep, setKeep] = useState<Readonly<Record<number, TextKeep>>>({});
  const answered = useRef(false);
  const answer = (run: () => void) => {
    if (answered.current) return;
    answered.current = true;
    run();
  };
  const choiceOf = (archive: number): TextKeep => keep[archive] ?? "json";
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) answer(onCancel);
      }}
      title="Texte auf zwei Seiten geändert"
      description="Diese Textarchive wurden als Text (JSON) und als Spieldatei geändert, seit das Studio sie zuletzt gebaut hat, etwa in DSPRE oder durch einen Git-Wechsel. Wähle je Archiv, welche Fassung gilt; die andere wird überschrieben."
      data={{ "data-text-conflict": "" }}
      actions={[
        { label: "Abbrechen", onClick: () => answer(onCancel) },
        { label: "Übernehmen und bauen", kind: "primary", onClick: () => answer(() => onResolve(archives.map((archive) => ({ archive, keep: choiceOf(archive) })))) },
      ]}
    >
      <ul className={styles.rows}>
        {archives.map((archive) => (
          <li key={archive} className={styles.row} data-archive={String(archive)}>
            <span className={styles.name} data-archive-name="">
              {archiveName(archive)}
            </span>
            <Segmented
              options={OPTIONS}
              value={choiceOf(archive)}
              onChange={(value) => setKeep((before) => ({ ...before, [archive]: value }))}
              ariaLabel={`Fassung von ${archiveName(archive)}`}
              size="s"
            />
          </li>
        ))}
      </ul>
    </Dialog>
  );
}
