import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { DataRowDto } from "../../api";
import { trainerRowLabel } from "../../data/trainer/trainerModel";
import { Button, InspectorRow, Picker, Reference } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import type { NavTarget } from "../navigation";
import { id4 } from "../terms";
import { placeableTrainers, scriptTrainerLabel, trainerOfScript } from "../trainerScript";
import styles from "./Editor.module.css";

/**
 * The object field „Trainer“: the trainer whose battle the script starts (3000 to 4999: `script − 3000 + 1`; from 5000 a double battle
 * partner) as a reference into Pokémon › Trainer, else „kein Trainer“. „Ändern…“ picks a trainer (never the dummy 0 nor the seen slot,
 * Platinum 929 / HeartGold 740): `onPick` gets its ID and writes the script (and the sightline of a figure that had none) in one change.
 * The trainer rows come from the data store: `needTrainers` asks for them when the field shows a trainer or the picker opens.
 */
export function TrainerField({
  script,
  family,
  trainers,
  needTrainers,
  goTo,
  onPick,
  disabled,
}: {
  script: number;
  family: string | null;
  trainers: readonly DataRowDto[] | null;
  needTrainers?: () => void;
  goTo: (target: NavTarget) => void;
  onPick: (id: number) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("workbench");
  const [picking, setPicking] = useState(false);
  const trainer = trainerOfScript(script);
  const hasTrainer = trainer !== null;
  useEffect(() => {
    if (hasTrainer || picking) needTrainers?.();
  }, [hasTrainer, picking, needTrainers]);
  const items = useMemo<PickerItem[]>(
    () => (picking && trainers ? placeableTrainers(trainers, family).map((row) => ({ id: row.index, label: trainerRowLabel(row), tech: row.tech, area: "pokemon" as const })) : []),
    [picking, trainers, family],
  );
  return (
    <>
      <InspectorRow label="Trainer">
        <div className={styles.inline} data-field="trainer">
          {trainer === null ? (
            <span className={styles.mono}>{t("trainerField.none")}</span>
          ) : (
            <Reference
              area="pokemon"
              label={scriptTrainerLabel(trainers, script) ?? ""}
              tech={id4(trainer.id)}
              onOpen={() => goTo({ kind: "trainer", index: trainer.id })}
              dataAttrs={{ "data-trainer": String(trainer.id) }}
            />
          )}
          <Button kind="quiet" size="s" aria-label={t("trainerField.change")} disabled={disabled} onClick={() => setPicking(true)}>
            {t("inspectorField.change")}
          </Button>
        </div>
      </InspectorRow>
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title={t("trainerField.choose")}
        items={items}
        placeholder={t("trainerField.search")}
        emptyText={trainers === null ? t("inspectorField.loading") : t("trainerField.notFound")}
        onPick={(item) => onPick(Number(item.id))}
      />
    </>
  );
}
