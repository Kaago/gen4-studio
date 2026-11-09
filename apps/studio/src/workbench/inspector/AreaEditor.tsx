import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { getAreaData, setAreaData } from "../../api";
import type { AreaDataDto, AssetId, MemberCounts } from "../../api";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { id4, termLabel } from "../terms";
import { areaFieldSpecs, AREA_FIELDS, areaValue } from "./areaFields";
import styles from "./Editor.module.css";
import { Field } from "./Field";
import { pickMax, shownFields } from "./fields";
import type { FieldSpec } from "./fields";
import { isDirty } from "./dirtyAssets";
import { EditSession } from "./headerCommit";
import { InspectorHead } from "./InspectorHead";
import { TechnicalSection } from "./TechnicalSection";

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
export function AreaEditor({ id, context, depth }: { id: number; context: AreaEditorContext; depth: Depth }) {
  const { t } = useTranslation("workbench");
  const { counts, dirty, revision, onChanged, onError } = context;
  const reset = useReset();
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
      <InspectorHead
        kind="look"
        title={`${termLabel("areaData")} ${id4(id)}`}
        unsaved={isDirty(dirty, "AreaData", id)}
        // The two textures the area data names (as edited, unsaved or not); the area data itself has no reset.
        actions={
          area === null
            ? undefined
            : [
                resetEntry({ kind: "map_texture", index: area.map_tileset }, reset.status, reset.open, t("areaEditor.resetMapTexture")),
                resetEntry({ kind: "building_texture", index: area.buildings_tileset }, reset.status, reset.open, t("areaEditor.resetBuildingTexture")),
              ]
        }
        depth={depth}
      />
      <div className={styles.body}>
        {area === null ? (
          <p className={styles.note}>{t("areaEditor.loading")}</p>
        ) : (
          <div key={refused}>
            {shownFields(areaFieldSpecs(area.family), depth).map((spec) => (
              <Field key={spec.key} spec={spec} value={areaValue(area, spec.key)} max={spec.kind === "pick" && spec.pick ? pickMax(spec.pick, counts) : undefined} onCommit={commit} showBase={depthAtLeast(depth, "tuefteln")} />
            ))}
          </div>
        )}
        {depthAtLeast(depth, "tuefteln") && area !== null && <TechnicalSection model={{ kind: "area", id }} source={{ kind: "area", area }} />}
        {!depthAtLeast(depth, "bauen") ? (
          <p className={styles.note}>{t("areaEditor.noteTell")}</p>
        ) : (
          <p className={styles.note}>{depthAtLeast(depth, "tuefteln") ? t("areaEditor.noteTinker") : t("areaEditor.noteBuild")}</p>
        )}
      </div>
    </div>
  );
}
