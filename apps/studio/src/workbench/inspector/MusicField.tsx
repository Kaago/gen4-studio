import { useEffect, useMemo, useState } from "react";
import { errorText, nameTables } from "../../api";
import type { ProjectInfo, SoundNameDto } from "../../api";
import { Button, InspectorRow, Picker } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import styles from "./Editor.module.css";
import type { FieldSpec } from "./fields";

/** The music pieces of each opened project, read once (they come from the family's command database, no edit changes them). */
const soundsOf = new WeakMap<ProjectInfo, Promise<readonly SoundNameDto[]>>();

/** The music pieces as read: `sounds` null while they are read or without a project; `error` why they could not be read. */
export interface SoundNames {
  sounds: readonly SoundNameDto[] | null;
  error: string | null;
}

const NOT_READ: SoundNames = { sounds: null, error: null };

/**
 * The music pieces of `project` (`name_tables`' `sounds`, sorted by id), or why they cannot be read (the fields then are number fields
 * that say so).
 */
export function useSoundNames(project: ProjectInfo | null): SoundNames {
  const [read, setRead] = useState<{ project: ProjectInfo; names: SoundNames } | null>(null);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    let request = soundsOf.get(project);
    if (request === undefined) {
      request = nameTables().then((tables) => tables.sounds);
      soundsOf.set(project, request);
      // A failed read is tried again by the next editor.
      request.catch(() => soundsOf.delete(project));
    }
    request.then(
      (sounds) => live && setRead({ project, names: { sounds, error: null } }),
      (e: unknown) => live && setRead({ project, names: { sounds: null, error: errorText(e) } }),
    );
    return () => {
      live = false;
    };
  }, [project]);
  return read !== null && read.project === project ? read.names : NOT_READ;
}

/** The piece with the number `id`; undefined when the table does not know it (or is not read). */
export const soundOf = (sounds: readonly SoundNameDto[] | null, id: number): SoundNameDto | undefined => sounds?.find((s) => s.id === id);

/**
 * A music field of Erzählen (`music_day_id`, `music_night_id`): the piece by its name and where the game plays it, „Wählen…“ opens a
 * picker of the pieces (name, where it plays as the description, the number as the technical part). A number the table does not know is
 * „Stück <n>“. A pick commits like any header field: one undoable header edit.
 */
export function MusicField({
  spec,
  value,
  sounds,
  onCommit,
}: {
  spec: FieldSpec;
  value: number;
  /** The pieces of the project; null while they are read. */
  sounds: readonly SoundNameDto[] | null;
  onCommit: (spec: FieldSpec, value: number) => void;
}) {
  const [picking, setPicking] = useState(false);
  const piece = soundOf(sounds, value);
  const items = useMemo<PickerItem[]>(
    () => (picking && sounds !== null ? sounds.map((s) => ({ id: s.id, label: s.name, tech: String(s.id), ...(s.used_in === null ? {} : { description: s.used_in }) })) : []),
    [picking, sounds],
  );
  return (
    <>
      <InspectorRow label={spec.label}>
        <div className={styles.inline} data-music-field={spec.key}>
          <span>{piece?.name ?? `Stück ${value}`}</span>
          {piece?.used_in && <span className={styles.pieceName}>{piece.used_in}</span>}
          <Button kind="quiet" size="s" aria-label={`${spec.label} wählen…`} onClick={() => setPicking(true)}>
            Wählen…
          </Button>
        </div>
      </InspectorRow>
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title={`${spec.label} wählen`}
        items={items}
        placeholder="Stück, Ort oder Nummer suchen"
        emptyText={sounds === null ? "Lädt …" : "Kein Stück gefunden"}
        onPick={(item) => onCommit(spec, Number(item.id))}
      />
    </>
  );
}
