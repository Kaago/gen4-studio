import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { EventKind } from "../../api";
import { Button, SelectField } from "../../ui/primitives";
import { EVENT_KINDS, KIND_LABELS } from "../../world/events";
import styles from "./AddEvent.module.css";

/** Picks the class of a new event and asks for it; it appears on the tile the pointer was last over. */
export function AddEvent({ disabled, onAdd }: { disabled?: boolean; onAdd: (kind: EventKind) => void }) {
  const { t } = useTranslation("workbench");
  const [kind, setKind] = useState<EventKind>("warp");
  // The labels are read here, in the language shown, never when the module loads.
  const options = EVENT_KINDS.map((k) => ({ value: k, label: KIND_LABELS[k] }));
  return (
    <span className={styles.add}>
      <SelectField label={t("addEvent.kind")} value={kind} options={options} disabled={disabled} onChange={setKind} />
      <Button kind="quiet" size="s" disabled={disabled} onClick={() => onAdd(kind)} title={t("addEvent.addTitle")}>
        {t("addEvent.add")}
      </Button>
    </span>
  );
}
