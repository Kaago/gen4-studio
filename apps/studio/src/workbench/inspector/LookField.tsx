import { useTranslation } from "react-i18next";
import type { HeaderSummary } from "../../api";
import { InspectorRow } from "../../ui/primitives";
import { LookPicker } from "../wizards/LookPicker";

/**
 * The header field „Aussehen“ of Erzählen: the look (area data) by the places that carry it, chosen in the picker of the wizards
 * („Aussehen wie …“) with `template`'s map drawn in each look; a pick is one undoable change of the field.
 */
export function LookField({
  value,
  headers,
  template,
  disabled,
  onCommit,
}: {
  value: number;
  headers: readonly HeaderSummary[];
  /** The header whose map shows each look: in the inspector the header itself. */
  template: HeaderSummary | null;
  disabled?: boolean;
  onCommit: (areaDataId: number) => void;
}) {
  const { t } = useTranslation("workbench");
  return (
    <InspectorRow label={t("lookField.label")}>
      <LookPicker headers={headers} template={template} value={value} onChange={onCommit} depth="erzaehlen" disabled={disabled} />
    </InspectorRow>
  );
}
