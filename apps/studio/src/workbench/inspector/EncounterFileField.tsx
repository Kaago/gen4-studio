import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { dataList, errorText } from "../../api";
import type { DataRowDto, HeaderFamily, HeaderSummary } from "../../api";
import { encounterRowLabel, rowUsers, usedByText } from "../../data/encounters/encounterModel";
import { Button, InspectorRow, Picker, Reference } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import type { NavTarget } from "../navigation";
import { id4 } from "../terms";
import styles from "./Editor.module.css";
import type { FieldSpec } from "./fields";

/** What `wild_pokemon` holds for „no encounter file“: Platinum (and Diamond/Pearl) a u16, HeartGold a u8. */
export const noEncounterFile = (family: HeaderFamily): number => (family === "HGSS" ? 0xff : 0xffff);

const NONE_ID = "none";

/**
 * The header field „Begegnungen“ (`wild_pokemon`): a reference to the encounter file in the Pokémon area, or „Keine“; „Ändern…“ opens a
 * picker of the files, each named by the places that use it, and „Keine“. The files are read when the picker opens (they change behind the
 * header editor: new files, splits); a tombstone (3d.4) is left out, the kernel refuses a new reference to it. Picking commits like any
 * header field: one undoable header edit.
 */
export function EncounterFileField({
  spec,
  value,
  family,
  headers,
  goTo,
  onCommit,
  disabled,
}: {
  spec: FieldSpec;
  value: number;
  family: HeaderFamily;
  headers: readonly HeaderSummary[];
  goTo: (target: NavTarget) => void;
  onCommit: (spec: FieldSpec, value: number) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("workbench");
  const none = noEncounterFile(family);
  const [picking, setPicking] = useState(false);
  const [files, setFiles] = useState<{ rows: DataRowDto[] | null; error: string | null } | null>(null);
  useEffect(() => {
    if (!picking) return;
    let current = true;
    setFiles(null);
    dataList("encounter").then(
      (rows) => current && setFiles({ rows, error: null }),
      (e: unknown) => current && setFiles({ rows: null, error: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }, [picking]);
  const items = useMemo<PickerItem[]>(() => {
    if (!picking || !files?.rows) return [];
    return [
      { id: NONE_ID, label: t("encounterFileField.none") },
      ...files.rows.filter((row) => row.deleted !== true).map((row) => {
        const users = rowUsers(row).length;
        return { id: row.index, label: encounterRowLabel(row, headers), tech: row.tech, area: "pokemon" as const, description: users === 0 ? t("encounterFileField.unused") : usedByText(users) };
      }),
    ];
  }, [picking, files, headers, t]);
  return (
    <>
      <InspectorRow label={spec.label}>
        <div className={styles.inline}>
          {value === none ? (
            <span className={styles.mono}>{t("encounterFileField.none")}</span>
          ) : (
            <Reference area="pokemon" label={spec.label} tech={id4(value)} onOpen={() => goTo({ kind: "encounter", index: value })} />
          )}
          <Button kind="quiet" size="s" aria-label={t("inspectorField.changeLabel", { label: spec.label })} disabled={disabled} onClick={() => setPicking(true)}>
            {t("inspectorField.change")}
          </Button>
        </div>
      </InspectorRow>
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title={t("inspectorField.chooseLabel", { label: spec.label })}
        items={items}
        placeholder={t("encounterFileField.search")}
        emptyText={files === null ? t("inspectorField.loading") : (files.error ?? t("encounterFileField.notFound"))}
        onPick={(item) => onCommit(spec, item.id === NONE_ID ? none : Number(item.id))}
      />
    </>
  );
}
