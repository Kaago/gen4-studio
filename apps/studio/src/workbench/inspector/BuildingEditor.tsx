import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, getBuildingGlb, getMap, updateBuilding } from "../../api";
import type { AssetId, BuildingDto, MemberCounts } from "../../api";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { useBuildingModelKind } from "../../original/useBuildingModelKind";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { t as translate } from "../../i18n";
import { Button, InspectorRow, Kbd, NumberField, TextField } from "../../ui/primitives";
import { id4, termLabel } from "../terms";
import { BUILDING_FIELDS, buildingBackend, buildingPositionFields, buildingScaleFields, parseNumberField } from "./buildingFields";
import type { BuildingFieldSpec } from "./buildingFields";
import styles from "./Editor.module.css";
import { isDirty } from "./dirtyAssets";
import { EditSession } from "./headerCommit";
import { InspectorHead } from "./InspectorHead";
import { TechnicalSection } from "./TechnicalSection";

/** What the building editor needs from App besides the building's map and index. */
export interface BuildingEditorContext {
  /** Area data of the shown scene; it selects the building texture set (and, on HGSS, interior or exterior models). */
  areaDataId: number;
  counts: MemberCounts;
  /** The assets with unsaved changes: the head says "Ungespeichert" while the building's map is one. */
  dirty: readonly AssetId[];
  /** Changes whenever buildings may have changed behind the editor's back (a drag, undo, redo). */
  revision: number;
  /** Changes whenever the area data may have changed (an edit, undo, redo): its area type picks the archive a model is reset in. */
  areaRevision?: number;
  /** A field was written: the scene reads the map's buildings again (and tries `retryModel` once more). */
  onChanged: (map: number, retryModel: number | null) => void | Promise<void>;
  /** Appends a building on the tile last hovered (or in `fallbackMap`) and selects it. */
  onAdd: (fallbackMap: number) => void | Promise<void>;
  /** Asks for confirmation, then deletes the building. */
  onDelete: (map: number, index: number) => void | Promise<void>;
  onError: (e: unknown) => void;
}

type Preview = { model: number; state: "loading" } | { model: number; state: "ok"; bytes: number } | { model: number; state: "error"; text: string };

/** The building with the given index of `map`, as the backend holds it, and how many buildings the map has. */
async function readBuilding(map: number, index: number): Promise<{ building: BuildingDto; count: number }> {
  const { buildings } = await getMap(map);
  const building = buildings[index];
  if (!building) throw new Error(translate("workbench:buildingEditor.gone", { n: index + 1, map }));
  return { building, count: buildings.length };
}

/** A decimal number (positions and scales are fractions of a tile): a text field that refuses what is not a number and says how to write one. */
function DecimalField({ label, value, onCommit }: { label: string; value: number; onCommit: (n: number) => void }) {
  const { t } = useTranslation("workbench");
  const [error, setError] = useState<string | undefined>();
  // A new value from the model makes an old refusal moot.
  useEffect(() => setError(undefined), [value]);
  return (
    <TextField
      label={label}
      value={String(value)}
      error={error}
      onChange={() => setError(undefined)}
      onCommit={(text) => {
        const n = parseNumberField(text);
        if (n === null) return setError(t("buildingEditor.notANumber"));
        setError(undefined);
        if (n !== value) onCommit(n);
      }}
    />
  );
}

/**
 * Editable form of one building: model number, position, scale (each committed field one undoable
 * change), the rotation as read-only hex (the game ignores it for map buildings, it is kept as it
 * is). Render it with `key` per (map, index): all reads and writes of an instance belong to that
 * one building (see `EditSession`).
 */
export function BuildingEditor({ map, index, context, depth }: { map: number; index: number; context: BuildingEditorContext; depth: Depth }) {
  const { t } = useTranslation("workbench");
  const { areaDataId, areaRevision = 0, counts, dirty, revision, onChanged, onAdd, onDelete, onError } = context;
  const reset = useReset();
  const modelKind = useBuildingModelKind(areaDataId, areaRevision);
  const [building, setBuilding] = useState<BuildingDto | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const [preview, setPreview] = useState<Preview | null>(null);
  // How many buildings the map has, for „Gebäude 3 von 5“.
  const [count, setCount] = useState<number | null>(null);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<EditSession<BuildingDto> | null>(null);

  useEffect(() => {
    const getBuilding = async (m: number, i: number) => {
      const read = await readBuilding(m, i);
      setCount(read.count);
      return read.building;
    };
    const s = new EditSession<BuildingDto>(index, buildingBackend(map, { getBuilding, update: updateBuilding }), BUILDING_FIELDS, {
      onState: setBuilding,
      onRefused: () => setRefused((n) => n + 1),
      onChanged: () => callbacks.current.onChanged(map, null),
      onError: (e) => callbacks.current.onError(e),
    });
    session.current = s;
    return () => s.dispose();
  }, [map, index]);

  useEffect(() => {
    void session.current?.load();
  }, [map, index, revision]);

  const modelId = building?.model_id ?? null;
  // Which model the form last asked the backend for, to show a result only for the current one.
  const asked = useRef<{ model: number; area: number } | null>(null);
  const check = (model: number) => {
    asked.current = { model, area: areaDataId };
    setPreview({ model, state: "loading" });
    getBuildingGlb(model, areaDataId).then(
      (glb) => {
        if (asked.current?.model === model) setPreview({ model, state: "ok", bytes: glb.byteLength });
      },
      (e) => {
        if (asked.current?.model === model) setPreview({ model, state: "error", text: errorText(e) });
      },
    );
  };
  // The scene loads the model by itself; this asks for it too (the backend shares the work) to say why it may be missing.
  useEffect(() => {
    if (modelId !== null) check(modelId);
    return () => {
      asked.current = null;
    };
  }, [modelId, areaDataId]);

  const commit = (key: string, label: string, value: number) => void session.current?.commit(key, label, value);
  const title = count === null ? t("buildingEditor.title", { n: index + 1 }) : t("buildingEditor.titleOf", { n: index + 1, count });
  const ident = `${termLabel("map")} ${id4(map)}`;
  if (building === null) {
    return (
      <div className={styles.panel}>
        <InspectorHead kind="building" title={title} ident={ident} unsaved={isDirty(dirty, "Map", map)} depth={depth} />
      </div>
    );
  }
  const modelMax = counts.BuildingModel === undefined ? undefined : Math.max(0, counts.BuildingModel - 1);

  const decimalRow = (spec: BuildingFieldSpec) => (
    <InspectorRow key={spec.key} label={spec.label}>
      <DecimalField label={spec.label} value={building[spec.key]} onCommit={(n) => commit(spec.key, spec.label, n)} />
    </InspectorRow>
  );
  const previewText =
    preview === null || preview.model !== building.model_id
      ? ""
      : preview.state === "loading"
        ? t("buildingEditor.modelLoading")
        : preview.state === "ok"
          ? t("buildingEditor.modelLoaded", { kb: Math.max(1, Math.round(preview.bytes / 1024)) })
          : t("buildingEditor.modelFailed", { error: preview.text });

  return (
    <div className={styles.panel}>
      <InspectorHead
        kind="building"
        title={title}
        ident={ident}
        unsaved={isDirty(dirty, "Map", map)}
        actions={[
          { label: t("eventEditor.delete"), detail: <Kbd keys={t("eventEditor.deleteKey")} />, onSelect: () => void onDelete(map, index) },
          "separator",
          // The building's model, from the archive the scene takes it from; while the area data is read that is not known yet.
          resetEntry({ kind: modelKind ?? "building_model", index: building.model_id }, modelKind === null ? null : reset.status, reset.open, t("buildingEditor.resetModel")),
        ]}
        depth={depth}
      />
      <div className={styles.body}>
        <div key={refused}>
          <InspectorRow label={t("buildingEditor.model")} hint={modelMax === undefined ? undefined : t("buildingEditor.modelRange", { max: modelMax })}>
            <NumberField label={t("buildingEditor.modelNumber")} value={building.model_id} bits={32} onCommit={(n) => commit("model_id", t("buildingEditor.model"), n)} showBase={depthAtLeast(depth, "tuefteln")} />
          </InspectorRow>
          <div className={styles.actions}>
            <Button kind="quiet" size="s" onClick={() => void Promise.resolve(onChanged(map, building.model_id)).then(() => check(building.model_id))} disabled={preview?.state === "loading"}>
              {t("buildingEditor.loadPreview")}
            </Button>
            <span className={preview?.state === "error" ? `${styles.note} ${styles.error}` : styles.note}>{previewText}</span>
          </div>
          {buildingPositionFields().map(decimalRow)}
          {buildingScaleFields().map(decimalRow)}
          <InspectorRow label={t("buildingEditor.rotation")} hint={t("buildingEditor.rotationReadOnly")}>
            <span className={styles.mono}>{building.rotation.join(" ")}</span>
          </InspectorRow>
        </div>
        {depthAtLeast(depth, "tuefteln") && <TechnicalSection model={{ kind: "building", map, index }} source={{ kind: "building", building }} />}
        <div className={styles.actions}>
          <Button kind="secondary" onClick={() => void onAdd(map)}>
            {t("buildingEditor.add")}
          </Button>
        </div>
        <p className={styles.note}>{t("buildingEditor.dragNote")}</p>
      </div>
    </div>
  );
}
