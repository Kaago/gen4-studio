import type { CSSProperties } from "react";
import type { BgDto, CoordDto, ObjectDto, WarpDto } from "../api";
import { eventDetails, eventTitle } from "../world/events";
import { tileSummary } from "../world/hover";
import type { TileInfo } from "../world/hover";
import { AreaEditor } from "./inspector/AreaEditor";
import type { AreaEditorContext } from "./inspector/AreaEditor";
import { BuildingEditor } from "./inspector/BuildingEditor";
import type { BuildingEditorContext } from "./inspector/BuildingEditor";
import { HeaderEditor } from "./inspector/HeaderEditor";
import type { HeaderEditorContext } from "./inspector/HeaderEditor";
import { MatrixCellEditor } from "./inspector/MatrixCellEditor";
import type { MatrixCellContext } from "./inspector/MatrixCellEditor";

/** What the inspector shows. */
export type InspectorModel =
  | { kind: "none" }
  /** The selected header, shown as an editable form. */
  | { kind: "header"; id: number }
  /** An area data record, shown as an editable form (opened from the header editor). */
  | { kind: "area"; id: number }
  /** A building selected in the scene, shown as an editable form. */
  | { kind: "building"; map: number; index: number }
  /** A cell of a matrix selected in the matrix panel, shown as an editable form. */
  | { kind: "matrixCell"; matrix: number; x: number; y: number }
  | {
      kind: "event";
      eventKind: "warp" | "object" | "coord" | "bg";
      index: number;
      data: WarpDto | ObjectDto | CoordDto | BgDto;
      /** Warps: location name (or internal name) of the destination header. */
      destName?: string;
    }
  | {
      kind: "tile";
      /** Map file index and the tile inside it (0..31). */
      map: number;
      x: number;
      y: number;
      /** Tile position in the whole matrix. */
      absX: number;
      absZ: number;
      info: TileInfo;
    };

const box: CSSProperties = { padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 14, color: "#4b5563" };

export function Inspector({
  model,
  headerEditor,
  matrixEditor,
  areaEditor,
  buildingEditor,
}: {
  model: InspectorModel;
  headerEditor: HeaderEditorContext;
  matrixEditor: MatrixCellContext;
  areaEditor: AreaEditorContext;
  buildingEditor: BuildingEditorContext;
}) {
  switch (model.kind) {
    case "header":
      return <HeaderEditor key={model.id} id={model.id} context={headerEditor} />;
    case "area":
      return <AreaEditor key={model.id} id={model.id} context={areaEditor} />;
    case "building":
      return <BuildingEditor key={`${model.map}:${model.index}`} map={model.map} index={model.index} context={buildingEditor} />;
    case "matrixCell":
      return <MatrixCellEditor key={`${model.matrix}:${model.x}:${model.y}`} matrix={model.matrix} x={model.x} y={model.y} context={matrixEditor} />;
    case "none":
      return <div style={box}>Nichts ausgewählt. Mit der Maus über ein Tile fahren oder einen Marker anklicken.</div>;
    case "event":
      return (
        <div style={box}>
          <div style={{ color: "#111827" }}>{eventTitle(model.eventKind, model.data, model.destName)}</div>
          <div style={{ marginTop: 4, fontSize: 12 }}>{eventDetails(model.eventKind, model.data)}</div>
        </div>
      );
    case "tile":
      return (
        <div style={box}>
          <div style={{ color: "#111827" }}>{tileSummary(model.map, model.x, model.y, model.info)}</div>
          <div style={{ marginTop: 4, fontSize: 12 }}>
            Matrix-Position x {model.absX}, z {model.absZ} · Kollision 0x
            {model.info.collision.toString(16).toUpperCase().padStart(2, "0")}
            {model.info.class ? ` · Klasse ${model.info.class}` : ""}
          </div>
        </div>
      );
  }
}
