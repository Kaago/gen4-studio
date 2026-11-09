import { useTranslation } from "react-i18next";
import { Checkbox } from "../ui/primitives";

/**
 * „Gelöschte zeigen“ in the head of a list (trainers, encounter files, the places of the world): off by default, the tombstones are left
 * out; on, they show with the badge „gelöscht“ and „Wiederherstellen“ in their context menu. `data-show-deleted` says whether it is on.
 */
export function ShowDeleted({ checked, onChange }: { checked: boolean; onChange: (checked: boolean) => void }) {
  const { t } = useTranslation("delete");
  return (
    <span data-show-deleted={checked ? "true" : "false"}>
      <Checkbox label={t("showDeleted.label")} checked={checked} onChange={onChange} />
    </span>
  );
}
