import type { BgDto, CoordDto, EventKind, ObjectDto, WarpDto } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { EmptyState } from "../ui/primitives";
import { eventDetails, eventTitle } from "../world/events";
import { tileSummary } from "../world/hover";
import type { TileInfo } from "../world/hover";
import { AreaEditor } from "./inspector/AreaEditor";
import type { AreaEditorContext } from "./inspector/AreaEditor";
import { BuildingEditor } from "./inspector/BuildingEditor";
import type { BuildingEditorContext } from "./inspector/BuildingEditor";
import { EventEditor } from "./inspector/EventEditor";
import type { EventEditorContext } from "./inspector/EventEditor";
import styles from "./inspector/Editor.module.css";
import { eventSubtitle } from "./inspector/eventFields";
import { HeaderEditor } from "./inspector/HeaderEditor";
import type { HeaderEditorContext } from "./inspector/HeaderEditor";
import { InspectorHead } from "./inspector/InspectorHead";
import { MatrixCellEditor } from "./inspector/MatrixCellEditor";
import type { MatrixCellContext } from "./inspector/MatrixCellEditor";
import { id4 } from "./terms";

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
  /** An event selected in the scene, shown as an editable form. Bound by event file, class and index. */
  | { kind: "event"; eventFile: number; eventKind: EventKind; index: number }
  /** A read-only summary of the event marker under the pointer. */
  | {
      kind: "eventHover";
      eventFile: number;
      eventKind: EventKind;
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

export function Inspector({
  model,
  headerEditor,
  matrixEditor,
  areaEditor,
  buildingEditor,
  eventEditor,
  depth,
}: {
  model: InspectorModel;
  headerEditor: HeaderEditorContext;
  matrixEditor: MatrixCellContext;
  areaEditor: AreaEditorContext;
  buildingEditor: BuildingEditorContext;
  eventEditor: EventEditorContext;
  /** Bauen, or Tüfteln: the section "Technisch" and the Dez | Hex switch of the number fields. */
  depth: Depth;
}) {
  switch (model.kind) {
    case "header":
      return <HeaderEditor key={model.id} id={model.id} context={headerEditor} depth={depth} />;
    case "area":
      return <AreaEditor key={model.id} id={model.id} context={areaEditor} depth={depth} />;
    case "building":
      return <BuildingEditor key={`${model.map}:${model.index}`} map={model.map} index={model.index} context={buildingEditor} depth={depth} />;
    case "event":
      return <EventEditor key={`${model.eventFile}:${model.eventKind}:${model.index}`} file={model.eventFile} kind={model.eventKind} index={model.index} context={eventEditor} depth={depth} />;
    case "matrixCell":
      return <MatrixCellEditor key={`${model.matrix}:${model.x}:${model.y}`} matrix={model.matrix} x={model.x} y={model.y} context={matrixEditor} depth={depth} />;
    case "none":
      return (
        <div className={styles.panel}>
          <EmptyState what="Nichts ausgewählt." why="Wähle links einen Ort, klicke in der Schau einen Marker an oder fahre mit der Maus über eine Kachel." />
        </div>
      );
    case "eventHover":
      return (
        <div className={styles.panel}>
          <InspectorHead title={eventTitle(model.eventKind, model.data, model.destName)} subtitle={eventSubtitle(model.eventKind, model.data.index, null, null)} viewOnly />
          <p className={styles.note}>{eventDetails(model.eventKind, model.data)}</p>
        </div>
      );
    case "tile":
      return (
        <div className={styles.panel}>
          <InspectorHead title={`Kachel x ${model.x}, y ${model.y}`} subtitle={`Karte ${id4(model.map)}`} viewOnly />
          <p className={styles.note}>{tileSummary(model.map, model.x, model.y, model.info)}</p>
          <p className={styles.note}>
            Matrix-Position x {model.absX}, z {model.absZ} · Kollision 0x{model.info.collision.toString(16).toUpperCase().padStart(2, "0")}
            {model.info.class ? ` · Klasse ${model.info.class}` : ""}
          </p>
        </div>
      );
  }
}
