import { trainerProblemFile } from "../../api";
import { t } from "../../i18n";
import { useDataDoc } from "../useDataDoc";
import type { DataDoc, DocSpec } from "../useDataDoc";
import { applyTrainer } from "./trainerModel";
import type { TrainerDoc } from "./trainerModel";

/**
 * A trainer is one member pair (properties and party) plus its name in the name bank and its battle texts: a `dataChanged` of the trainer
 * (the text tables announce any trainer), a `textData` of either bank and an `eventData` (its figures in the world) read it again. Its
 * battle text warning goes to the Problems list with every read.
 */
const TRAINER: DocSpec<TrainerDoc> = {
  area: "pokemon",
  kind: "trainer",
  parts: ["trainer"],
  accept: (doc) => (doc.kind === "trainer" ? doc : null),
  get wrongKind() {
    return t("data:trainer.wrongKind");
  },
  apply: applyTrainer,
  texts: (doc) => [doc.name_archive, doc.text_archive],
  problems: (doc, index) => ({ file: trainerProblemFile(index), rows: doc.problems }),
  // „Steht in der Welt“: placing, moving, retargeting or undoing a figure in the world changes the trainer's events.
  rereadOn: ["eventData"],
};

/** Trainer `index` (null: none) with its queue of edits (`useDataDoc`): every edit built from the trainer as the edits before it left it. */
export const useTrainer = (index: number | null): DataDoc<TrainerDoc> => useDataDoc(TRAINER, index);
