import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { FlagKind, ProjectInfo } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Button, InspectorRow, NumberField, Reference } from "../ui/primitives";
import { FlagPicker } from "./FlagPicker";
import { displayName, hex4, noneText } from "./flagWords";
import { useFlagList } from "./useFlagList";
import styles from "./FlagField.module.css";

export type FlagFieldProps = {
  kind: FlagKind;
  label: string;
  /** The number the record holds; 0 is none („Keins“) and a valid value. */
  value: number;
  /** A picked or typed number, 0 included. */
  onCommit: (n: number) => void;
  disabled?: boolean;
  depth: Depth;
  project: ProjectInfo | null;
  /** Changes when the names or uses may have changed: the name is read again. */
  revision: number;
  onShowInDirectory?: (number: number) => void;
  /** The `data-flag-field` of the name chip (the record's key: `hidden_flag`, `var:1`); the kind when omitted. */
  field?: string;
  /** A sentence under the field (the row's hint). */
  hint?: string;
  /** Without the labelled inspector row: the chip, „Wählen…“ and the number field alone (a cell of a table). */
  bare?: boolean;
  /** What `revision` counts (`events`, `scripts`): lists read under different counters are not shared. */
  source?: string;
};

/**
 * A flag or a variable of a record: its name as a chip of the area Geschichten (own name, else pret name, else kind and number; the number
 * in mono), which opens the picker like „Wählen…“ next to it; in Tüfteln the number field with Dez | Hex below. 0 shows „Keins“.
 */
export function FlagField({ kind, label, value, onCommit, disabled, depth, project, revision, onShowInDirectory, field, hint, bare, source }: FlagFieldProps) {
  const { t } = useTranslation("flags");
  const [picking, setPicking] = useState(false);
  // 0 needs no name: a figure without a flag does not build the reference index (the picker reads when it opens).
  const { list } = useFlagList(kind, { project: value === 0 ? null : project, revision, source });
  const entry = value === 0 ? undefined : list?.entries.find((e) => e.number === value);
  const name = value === 0 ? noneText() : displayName(entry ?? { name: null, own_id: null }, kind, value);
  const control = (
    <div className={styles.field}>
      <div className={styles.line}>
        <Reference
          area="geschichten"
          label={name}
          tech={value === 0 ? undefined : hex4(value)}
          onOpen={() => setPicking(true)}
          disabled={disabled}
          dataAttrs={{ "data-flag-field": field ?? kind }}
        />
        <Button kind="quiet" size="s" aria-label={t("flagField.chooseLabel", { label })} disabled={disabled} onClick={() => setPicking(true)}>
          {t("flagField.choose")}
        </Button>
      </div>
      {depthAtLeast(depth, "tuefteln") && <NumberField label={label} value={value} max={0xffff} bits={16} disabled={disabled} onCommit={onCommit} showBase />}
      <FlagPicker open={picking} kind={kind} value={value} onOpenChange={setPicking} onPick={onCommit} onShowInDirectory={onShowInDirectory} project={project} revision={revision} />
    </div>
  );
  if (bare) return control;
  return (
    <InspectorRow label={label} hint={hint}>
      {control}
    </InspectorRow>
  );
}
