import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { errorText, getBuildingGlb, getMap, updateBuilding } from "../../api";
import type { BuildingDto, MemberCounts } from "../../api";
import { DraftInput } from "../DraftInput";
import { BUILDING_FIELDS, BUILDING_POSITION_FIELDS, BUILDING_SCALE_FIELDS, buildingBackend, parseNumberField } from "./buildingFields";
import type { BuildingFieldSpec } from "./buildingFields";
import { EditSession } from "./headerCommit";

/** What the building editor needs from App besides the building's map and index. */
export interface BuildingEditorContext {
  /** Area data of the shown scene; it selects the building texture set (and, on HGSS, interior or exterior models). */
  areaDataId: number;
  counts: MemberCounts;
  /** Changes whenever buildings may have changed behind the editor's back (a drag, undo, redo). */
  revision: number;
  /** A field was written: the scene reads the map's buildings again (and tries `retryModel` once more). */
  onChanged: (map: number, retryModel: number | null) => void | Promise<void>;
  /** Appends a building on the tile last hovered (or in `fallbackMap`) and selects it. */
  onAdd: (fallbackMap: number) => void | Promise<void>;
  /** Asks for confirmation, then deletes the building. */
  onDelete: (map: number, index: number) => void | Promise<void>;
  onError: (e: unknown) => void;
}

const row: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 96px", alignItems: "center", columnGap: 8, padding: "2px 0" };
const input: CSSProperties = { width: "100%", fontFamily: "ui-monospace, monospace" };
const muted: CSSProperties = { fontSize: 12, color: "#6b7280" };

type Preview = { model: number; state: "loading" } | { model: number; state: "ok"; bytes: number } | { model: number; state: "error"; text: string };

/** The building with the given index of `map`, as the backend holds it. */
async function getBuilding(map: number, index: number): Promise<BuildingDto> {
  const building = (await getMap(map)).buildings[index];
  if (!building) throw new Error(`Gebäude ${index} existiert in Karte ${map} nicht (mehr)`);
  return building;
}

/**
 * Editable form of one building: model number, position, scale (each committed field one undoable
 * change), the rotation as read-only hex (the game ignores it for map buildings, it is kept as it
 * is). Render it with `key` per (map, index): all reads and writes of an instance belong to that
 * one building (see `EditSession`).
 */
export function BuildingEditor({ map, index, context }: { map: number; index: number; context: BuildingEditorContext }) {
  const { areaDataId, counts, revision, onChanged, onAdd, onDelete, onError } = context;
  const [building, setBuilding] = useState<BuildingDto | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const [preview, setPreview] = useState<Preview | null>(null);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<EditSession<BuildingDto> | null>(null);

  useEffect(() => {
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
  const commitModel = (text: string) => {
    const v = parseNumberField(text);
    if (v !== null) commit("model_id", "Modell", Math.min(0xffff_ffff, Math.max(0, Math.round(v))));
  };
  const commitNumber = (spec: BuildingFieldSpec) => (text: string) => {
    const v = parseNumberField(text);
    if (v !== null) commit(spec.key, spec.label, v);
  };

  const title = `Gebäude ${index} · Karte ${String(map).padStart(4, "0")}`;
  if (building === null) return <div style={{ padding: 16, fontSize: 13, color: "#6b7280" }}>{title}</div>;
  const modelMax = counts.BuildingModel === undefined ? undefined : Math.max(0, counts.BuildingModel - 1);

  const numberRow = (spec: BuildingFieldSpec) => (
    <label key={spec.key} style={row}>
      <span>{spec.label}</span>
      <DraftInput label={spec.label} type="number" step={spec.step} value={String(building[spec.key])} onCommit={commitNumber(spec)} style={input} />
    </label>
  );

  return (
    <div style={{ padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={{ fontSize: 14, fontWeight: 600 }}>{title}</div>
      <div key={refused} style={{ marginTop: 8 }}>
        <label style={row}>
          <span>Modell</span>
          <DraftInput
            label={modelMax === undefined ? "Modellnummer" : `Modellnummer (außen 0 bis ${modelMax})`}
            type="number"
            min={0}
            value={String(building.model_id)}
            onCommit={commitModel}
            style={input}
          />
        </label>
        <div style={{ display: "flex", alignItems: "center", gap: 8, margin: "2px 0 6px" }}>
          <button onClick={() => void Promise.resolve(onChanged(map, building.model_id)).then(() => check(building.model_id))} disabled={preview?.state === "loading"}>
            Vorschau laden
          </button>
          <span style={{ ...muted, color: preview?.state === "error" ? "#b91c1c" : "#6b7280" }}>
            {preview === null || preview.model !== building.model_id
              ? ""
              : preview.state === "loading"
                ? "Lade Modell ..."
                : preview.state === "ok"
                  ? `Modell geladen (${Math.max(1, Math.round(preview.bytes / 1024))} KB)`
                  : `Modell nicht geladen, die 3D-Ansicht zeigt eine Box: ${preview.text}`}
          </span>
        </div>
        {BUILDING_POSITION_FIELDS.map(numberRow)}
        {BUILDING_SCALE_FIELDS.map(numberRow)}
        <div style={{ ...row, marginTop: 4 }} title="Das Spiel ignoriert die Rotation von Karten-Gebäuden; sie bleibt unverändert erhalten.">
          <span>Rotation (nur lesbar)</span>
          <span style={{ fontFamily: "ui-monospace, monospace", fontSize: 11 }}>{building.rotation.join(" ")}</span>
        </div>
      </div>
      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button onClick={() => void onAdd(map)}>Neues Gebäude</button>
        <button onClick={() => void onDelete(map, index)}>Löschen</button>
      </div>
      <p style={{ marginTop: 8, ...muted }}>
        Ziehen in der 3D-Ansicht verschiebt das Gebäude in Halb-Kachel-Schritten. Neue Gebäude entstehen auf der zuletzt berührten Kachel; Entf löscht das gewählte Gebäude nach Rückfrage.
      </p>
    </div>
  );
}
