import { useEffect, useRef, useState } from "react";
import { errorText, freeFlag } from "../../api";
import type { ProjectInfo } from "../../api";
import { useStableCallback } from "../../app/useStable";
import { FlagField } from "../../flags/FlagField";
import { MessageEditor } from "../../workbench/text/MessageEditor";
import { TextPreview } from "../../workbench/wizards/TextPreview";
import styles from "./StoryRecipeDialog.module.css";

/** What a figure or a scene says: the text (Enter a new line, Shift+Enter a new page) and the DS box with its pages beside it. */
export function SaysField({ value, onChange, disabled }: { value: string; onChange: (text: string) => void; disabled?: boolean }) {
  return (
    <div className={styles.field}>
      <span className={styles.label}>Text</span>
      <div className={styles.preview}>
        <MessageEditor label="Text" value={value} onCommit={async () => true} onDraft={onChange} disabled={disabled} />
      </div>
      <p className={styles.note}>Enter: neue Zeile; Umschalt+Enter: neue Seite (▼).</p>
      <div className={styles.preview}>
        <TextPreview text={value} />
      </div>
    </div>
  );
}

/** „Noch 20 Flags frei“, „Noch 1 Flag frei“. */
const freeFlagsText = (n: number): string => `Noch ${n} ${n === 1 ? "Flag" : "Flags"} frei`;

/**
 * The flag a recipe takes from the pool (an appearance, a reward): opening asks the pool for its smallest free flag and puts it into the
 * field while the field shows none (it stays changeable), with the line „Noch {n} Flags frei“; a pool that cannot give one says why.
 */
export function PoolFlagField({ label, value, onChange, project, revision, disabled }: { label: string; value: number; onChange: (n: number) => void; project: ProjectInfo | null; revision: number; disabled?: boolean }) {
  const [pool, setPool] = useState<{ free: number } | { refusal: string } | null>(null);
  const take = useStableCallback(onChange);
  // The field's number now: an answer that comes after a pick does not overwrite it.
  const current = useRef(value);
  current.current = value;
  useEffect(() => {
    let live = true;
    freeFlag("flag").then(
      (dto) => {
        if (!live) return;
        setPool({ free: dto.pool_free });
        if (current.current === 0) take(dto.number);
      },
      (e: unknown) => live && setPool({ refusal: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [take]);
  return (
    <div className={styles.field}>
      <span className={styles.label}>{label}</span>
      <FlagField bare kind="flag" label={label} value={value} onCommit={onChange} disabled={disabled} depth="bauen" project={project} revision={revision} field="story-flag" />
      {pool !== null && (
        <p className={styles.note} data-pool-free="">
          {"free" in pool ? freeFlagsText(pool.free) : pool.refusal}
        </p>
      )}
    </div>
  );
}
