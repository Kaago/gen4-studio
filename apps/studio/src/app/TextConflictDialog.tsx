import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TextKeep } from "../api";
import { Dialog, Notice, Segmented } from "../ui/primitives";
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
  /** The choice is being written: the dialog waits (no button, no choice, no closing) and says so until the build starts. */
  busy?: boolean;
};


/**
 * The question a build puts when text archives changed as JSON and as member since the studio last built them (spec 4.2 „Konflikt beim
 * Bauen“): per archive which version applies, the JSON by default. Nothing is overwritten silently: „Abbrechen“ builds nothing. Answered
 * once: the Escape of the closing dialog after „Übernehmen und bauen“ cancels nothing.
 */
export function TextConflictDialog({ archives, onResolve, onCancel, busy = false }: TextConflictDialogProps) {
  const { t } = useTranslation("app");
  const archiveName = (archive: number) => t("textConflict.archive", { id: String(archive).padStart(4, "0") });
  const [keep, setKeep] = useState<Readonly<Record<number, TextKeep>>>({});
  const answered = useRef(false);
  const answer = (run: () => void) => {
    if (answered.current) return;
    answered.current = true;
    run();
  };
  const choiceOf = (archive: number): TextKeep => keep[archive] ?? "json";
  const options: readonly SegmentedOption<TextKeep>[] = [
    { value: "json", label: t("textConflict.json"), disabled: busy },
    { value: "member", label: t("textConflict.member"), disabled: busy },
  ];
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) answer(onCancel);
      }}
      title={t("textConflict.title")}
      description={t("textConflict.description")}
      data={{ "data-text-conflict": "" }}
      actions={[
        { label: t("textConflict.cancel"), onClick: () => answer(onCancel), disabled: busy },
        { label: t("textConflict.resolve"), kind: "primary", onClick: () => answer(() => onResolve(archives.map((archive) => ({ archive, keep: choiceOf(archive) })))), disabled: busy },
      ]}
    >
      <ul className={styles.rows}>
        {archives.map((archive) => (
          <li key={archive} className={styles.row} data-archive={String(archive)}>
            <span className={styles.name} data-archive-name="">
              {archiveName(archive)}
            </span>
            <Segmented
              options={options}
              value={choiceOf(archive)}
              onChange={(value) => setKeep((before) => ({ ...before, [archive]: value }))}
              ariaLabel={t("textConflict.versionOf", { archive: archiveName(archive) })}
              size="s"
            />
          </li>
        ))}
      </ul>
      {busy && (
        <Notice kind="info">{t("textConflict.busy")}</Notice>
      )}
    </Dialog>
  );
}
