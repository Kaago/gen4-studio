import { useMemo, useState } from "react";
import type { FigureRef, HeaderSummary, ObjectDto } from "../../api";
import { Button, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { headerLabel } from "../../workbench/headers";
import { HeaderPicker } from "../../workbench/inspector/HeaderPicker";
import { trainerOfScript } from "../../workbench/trainerScript";
import { useEvents } from "../../workbench/useEvents";
import { TRAINERS_TALK } from "./storyRecipeModel";
import styles from "./StoryRecipeDialog.module.css";

export type FigureFieldProps = {
  /** The place of the figure (null: none chosen yet). */
  header: number | null;
  /** The figure's `local_id` in the place's event file (null: none chosen yet). */
  localId: number | null;
  /** A figure picked (null: another place was chosen, its figure is not chosen yet). */
  onChange: (figure: FigureRef | null) => void;
  headers: readonly HeaderSummary[];
  /** Changes when the events may have changed: the figures are read again. */
  revision: number;
  /** Talk and a reward: a trainer is listed with its reason but cannot be picked. */
  lockTrainers?: boolean;
  disabled?: boolean;
  onError: (e: unknown) => void;
};

/**
 * A trainer as the kernel tells it (`story_common`, the refusal of talk and a reward): an object that sees (`trainer_type` ≠ 0) or runs
 * a trainer's battle script (3000 to 4999; a double battle partner's script from 5000 is not one).
 */
export const isTrainerFigure = (o: Pick<ObjectDto, "script" | "trainer_type">): boolean => o.trainer_type !== 0 || trainerOfScript(o.script)?.partner === false;

/** „Figur 3 · Aussehen 97“: a figure by its local id and its look. */
export const figureLabel = (o: Pick<ObjectDto, "local_id" | "graphics_id">): string => `Figur ${o.local_id} · Aussehen ${o.graphics_id}`;

/**
 * The figure of a step recipe: its place (the header picker) and the figure among the objects of the place's event file (a picker, „Figur
 * {local_id} · Aussehen {graphics_id}“; a trainer carries the badge „Trainer“ and, for talk and a reward, is listed but locked: trainers
 * speak through their battle texts). Choosing another place clears the figure.
 */
export function FigureField({ header, localId, onChange, headers, revision, lockTrainers = false, disabled, onError }: FigureFieldProps) {
  // The place chosen here, until the owner names another one (a figure picked in another place).
  const [place, setPlace] = useState(header);
  const [seen, setSeen] = useState(header);
  if (seen !== header) {
    setSeen(header);
    if (header !== null) setPlace(header);
  }
  const [picking, setPicking] = useState<"place" | "figure" | null>(null);
  const shown = headers.find((h) => h.id === place);
  const events = useEvents(shown?.event_file_id ?? null, onError, revision);
  const objects = useMemo(() => events?.objects ?? [], [events]);
  const chosen = localId === null || place !== header ? undefined : objects.find((o) => o.local_id === localId);
  const items = useMemo<PickerItem[]>(
    () =>
      picking !== "figure"
        ? []
        : objects.map((o) => {
            const trainer = isTrainerFigure(o);
            const locked = trainer && lockTrainers;
            return {
              id: o.index,
              label: figureLabel(o),
              area: "welt",
              count: trainer ? "Trainer" : undefined,
              countTitle: trainer && lockTrainers ? TRAINERS_TALK : undefined,
              description: locked ? TRAINERS_TALK : undefined,
              disabled: locked,
            };
          }),
    [picking, objects, lockTrainers],
  );
  const figureText = chosen !== undefined ? figureLabel(chosen) : localId !== null && place === header ? `Figur ${localId}` : "Keine gewählt";
  return (
    <>
      <div className={styles.field}>
        <span className={styles.label}>Ort</span>
        <div className={styles.line}>
          <span data-figure-place="">{shown === undefined ? "Keiner gewählt" : headerLabel(shown)}</span>
          <Button kind="quiet" size="s" aria-label="Ort wählen…" disabled={disabled} onClick={() => setPicking("place")}>
            Wählen…
          </Button>
        </div>
      </div>
      <div className={styles.field}>
        <span className={styles.label}>Figur</span>
        <div className={styles.line}>
          <span data-figure-pick="">{figureText}</span>
          <Button kind="quiet" size="s" aria-label="Figur wählen…" disabled={disabled || shown === undefined} onClick={() => setPicking("figure")}>
            Wählen…
          </Button>
        </div>
        {shown !== undefined && events !== null && objects.length === 0 && <p className={styles.note}>Der Ort hat keine Figuren.</p>}
      </div>
      <HeaderPicker
        open={picking === "place"}
        onOpenChange={(open) => !open && setPicking(null)}
        headers={headers}
        title="Ort wählen"
        onPick={(id) => {
          if (id === place) return;
          setPlace(id);
          onChange(null);
        }}
      />
      <Picker
        open={picking === "figure"}
        onOpenChange={(open) => !open && setPicking(null)}
        title="Figur wählen"
        items={items}
        placeholder="Figur oder Nummer suchen"
        emptyText="Keine Figur an diesem Ort."
        onPick={(item) => {
          const object = objects.find((o) => o.index === Number(item.id));
          if (object !== undefined && place !== null) onChange({ header: place, local_id: object.local_id });
        }}
      />
    </>
  );
}
