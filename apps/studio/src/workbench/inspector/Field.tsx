import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { HeaderSummary } from "../../api";
import { Button, InspectorRow, NumberField, Picker, Reference, SelectField, Switch } from "../../ui/primitives";
import type { NavTarget } from "../navigation";
import styles from "./Editor.module.css";
import { fieldBits } from "./fields";
import type { FieldSpec } from "./fields";
import { pickerItems, referenceFor } from "./referenceFor";

export interface FieldProps {
  spec: FieldSpec;
  value: number;
  /** Highest valid index of a `pick` field (`pickMax`); undefined while the archive's size is not known. */
  max: number | undefined;
  onCommit: (spec: FieldSpec, value: number) => void;
  disabled?: boolean;
  /** For the names a reference shows (the event file's header). */
  headers?: readonly HeaderSummary[];
  /** Where a click on a reference leads. */
  goTo?: (target: NavTarget) => void;
  /** Replaces the jump of a reference (script files open in the script panel, and only once rotom has them). */
  open?: { run: () => void; disabled?: boolean };
  /** More controls after the field (the buttons next to a script number). */
  extra?: ReactNode;
  /** A number field shows the Dez | Hex switch (the depth Tüfteln). */
  showBase?: boolean;
}

const NO_HEADERS: readonly HeaderSummary[] = [];

/**
 * One record field as an inspector row. Numbers are committed on blur or Enter, never on every keystroke, and an entry
 * the field cannot take is refused with its limit instead of being clamped (see `NumberField`); a switch and a select
 * commit on the change. A `pick` field is a reference to the record the number names plus „Ändern…“, which opens a picker
 * over 0 to `max`; while `max` is unknown it is a plain number field.
 */
export function Field({ spec, value, max, onCommit, disabled, headers = NO_HEADERS, goTo, open, extra, showBase }: FieldProps) {
  const { t } = useTranslation("workbench");
  const [picking, setPicking] = useState(false);
  const items = useMemo(() => (picking && max !== undefined ? pickerItems(spec, max, headers) : []), [picking, spec, max, headers]);

  if (spec.kind === "bool") {
    return (
      <InspectorRow label={spec.label}>
        <Switch label={spec.label} checked={value !== 0} onChange={(on) => onCommit(spec, on ? 1 : 0)} disabled={disabled} />
      </InspectorRow>
    );
  }
  if (spec.kind === "select") {
    return (
      <InspectorRow label={spec.label}>
        <SelectField label={spec.label} value={value} options={spec.options ?? []} onChange={(next) => onCommit(spec, next)} disabled={disabled} />
      </InspectorRow>
    );
  }
  if (spec.kind === "pick" && max !== undefined) {
    const reference = referenceFor(spec, value, headers);
    const target = reference.target;
    return (
      <>
        <InspectorRow label={spec.label} hint={t("inspectorField.range", { max })}>
          <div className={styles.inline}>
            {open || target !== null ? (
              <Reference
                area={reference.area}
                label={reference.label}
                tech={reference.tech}
                onOpen={open ? open.run : () => target && goTo?.(target)}
                disabled={open?.disabled}
              />
            ) : (
              <span className={styles.mono}>{reference.tech}</span>
            )}
            <Button kind="quiet" size="s" aria-label={t("inspectorField.changeLabel", { label: spec.label })} disabled={disabled} onClick={() => setPicking(true)}>
              {t("inspectorField.change")}
            </Button>
            {extra}
          </div>
        </InspectorRow>
        <Picker
          open={picking}
          onOpenChange={setPicking}
          title={t("inspectorField.chooseLabel", { label: spec.label })}
          items={items}
          placeholder={t("inspectorField.searchNumber")}
          onPick={(item) => onCommit(spec, Number(item.id))}
        />
      </>
    );
  }
  return (
    <InspectorRow label={spec.label}>
      <div className={styles.inline}>
        <div className={styles.grow}>
          <NumberField label={spec.label} value={value} bits={fieldBits(spec)} max={spec.kind === "pick" ? max : undefined} onCommit={(n) => onCommit(spec, n)} disabled={disabled} showBase={showBase} />
        </div>
        {extra}
      </div>
    </InspectorRow>
  );
}
