import { useEffect, useRef, useState } from "react";
import { getAreaData, setAreaData } from "../../api";
import type { AreaDataDto, AssetId, MemberCounts } from "../../api";
import { id4 } from "../terms";
import { areaFieldSpecs, AREA_FIELDS, areaValue } from "./areaFields";
import styles from "./Editor.module.css";
import { Field } from "./Field";
import { pickMax } from "./fields";
import type { FieldSpec } from "./fields";
import { isDirty } from "./dirtyAssets";
import { EditSession } from "./headerCommit";
import { InspectorHead } from "./InspectorHead";

/** What the area data editor needs from App besides the area data id. */
export interface AreaEditorContext {
  counts: MemberCounts;
  /** The assets with unsaved changes: the head says "Ungespeichert" while this area data is one. */
  dirty: readonly AssetId[];
  /** Changes whenever the area data may have changed behind the editor's back (undo, redo, reload). */
  revision: number;
  /** Called after a field was written, so the scene's textures and the dirty count follow. */
  onChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/**
 * Editable form of one area data record (tilesets and light); every committed field is one
 * undoable change. Render it with `key={id}`: all reads and writes of an instance belong to its
 * one record (see `EditSession`).
 */
export function AreaEditor({ id, context }: { id: number; context: AreaEditorContext }) {
  const { counts, dirty, revision, onChanged, onError } = context;
  const [area, setArea] = useState<AreaDataDto | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<EditSession<AreaDataDto> | null>(null);

  useEffect(() => {
    const s = new EditSession<AreaDataDto>(
      id,
      { get: getAreaData, set: setAreaData },
      AREA_FIELDS,
      {
        onState: setArea,
        onRefused: () => setRefused((n) => n + 1),
        onChanged: () => callbacks.current.onChanged(),
        onError: (e) => callbacks.current.onError(e),
      },
    );
    session.current = s;
    return () => s.dispose();
  }, [id]);

  useEffect(() => {
    void session.current?.load();
  }, [id, revision]);

  const commit = (spec: FieldSpec, value: number) => void session.current?.commit(spec.key, spec.label, value);

  return (
    <div className={styles.panel}>
      <InspectorHead title={`Area-Data ${id4(id)}`} subtitle="Tilesets und Licht" unsaved={isDirty(dirty, "AreaData", id)} />
      <div className={styles.body}>
        {area === null ? (
          <p className={styles.note}>Lade Area-Data …</p>
        ) : (
          <div key={refused}>
            {areaFieldSpecs(area.family).map((spec) => (
              <Field key={spec.key} spec={spec} value={areaValue(area, spec.key)} max={spec.kind === "pick" && spec.pick ? pickMax(spec.pick, counts) : undefined} onCommit={commit} />
            ))}
          </div>
        )}
        <p className={styles.note}>
          Das Karten-Tileset bestimmt die Texturen aller Karten, deren Header diese Area-Data verwendet; die 3D-Ansicht lädt die Modelle nach einer Änderung neu.
        </p>
      </div>
    </div>
  );
}
