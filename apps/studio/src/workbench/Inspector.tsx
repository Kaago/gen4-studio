import type { ReactNode } from "react";
import type { BgDto, CheckProblemDto, CoordDto, EventKind, ObjectDto, WarpDto } from "../api";
import type { ChecksApi } from "../checks/useChecks";
import type { Depth } from "../ui/frame/shellState";
import { EmptyState } from "../ui/primitives";
import type { TileInfo } from "../world/hover";
import { AreaEditor } from "./inspector/AreaEditor";
import type { AreaEditorContext } from "./inspector/AreaEditor";
import { BuildingEditor } from "./inspector/BuildingEditor";
import type { BuildingEditorContext } from "./inspector/BuildingEditor";
import { ChecksSection } from "./inspector/ChecksSection";
import { EventEditor } from "./inspector/EventEditor";
import type { EventEditorContext } from "./inspector/EventEditor";
import type { EventLive } from "./inspector/eventBreakpoint";
import styles from "./inspector/Editor.module.css";
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
  /** An event selected in the scene, shown as an editable form. Bound by event file, class and index. */
  | { kind: "event"; eventFile: number; eventKind: EventKind; index: number }
  /** The event marker under the pointer (a hover: the status line names it, the inspector has no view of it). */
  | {
      kind: "eventHover";
      eventFile: number;
      eventKind: EventKind;
      index: number;
      data: WarpDto | ObjectDto | CoordDto | BgDto;
      /** Warps: the name of the destination header as the world tree calls it (undefined: unknown header). */
      destName?: string;
    }
  /** The tile under the pointer (a hover, like the marker). */
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
      /** BDHC height of the tile centre in world units (null: none, see `Ground.units`). */
      units: number | null;
    };

/** What the scene reports under the pointer: the middle of the status line names it while the inspector keeps the selection. */
export type SceneHover = Extract<InspectorModel, { kind: "eventHover" | "tile" }>;

const NO_FINDINGS: readonly CheckProblemDto[] = [];

export function Inspector({
  model,
  headerEditor,
  matrixEditor,
  areaEditor,
  buildingEditor,
  eventEditor,
  depth,
  eventLive,
  findings = NO_FINDINGS,
  checks = null,
}: {
  model: InspectorModel;
  headerEditor: HeaderEditorContext;
  matrixEditor: MatrixCellContext;
  areaEditor: AreaEditorContext;
  buildingEditor: BuildingEditorContext;
  eventEditor: EventEditorContext;
  /** Bauen, or Tüfteln: the section "Technisch" and the Dez | Hex switch of the number fields. */
  depth: Depth;
  /** Play mode: the event editor's row „Haltepunkt“. */
  eventLive?: EventLive;
  /** What the checks found at the record shown (`findingsFor`): the section „Prüfungen“ below its editor. */
  findings?: readonly CheckProblemDto[];
  checks?: ChecksApi | null;
}) {
  /**
   * An editor with the section „Prüfungen“ below it while the checks found something at its record. The frame is always the same, so the
   * editor is never mounted anew when findings come or go (it would lose its loaded record, the focus and an open picker).
   */
  const withChecks = (editor: ReactNode) => (
    <div className={styles.withChecks}>
      <div className={styles.editorPart}>{editor}</div>
      {findings.length > 0 && (
        <div className={styles.checksPart}>
          <ChecksSection findings={findings} checks={checks} depth={depth} />
        </div>
      )}
    </div>
  );
  switch (model.kind) {
    case "header":
      return withChecks(<HeaderEditor key={model.id} id={model.id} context={headerEditor} depth={depth} />);
    case "area":
      return withChecks(<AreaEditor key={model.id} id={model.id} context={areaEditor} depth={depth} />);
    case "building":
      return withChecks(<BuildingEditor key={`${model.map}:${model.index}`} map={model.map} index={model.index} context={buildingEditor} depth={depth} />);
    case "event":
      return withChecks(<EventEditor key={`${model.eventFile}:${model.eventKind}:${model.index}`} file={model.eventFile} kind={model.eventKind} index={model.index} context={eventEditor} depth={depth} live={eventLive} />);
    case "matrixCell":
      return withChecks(<MatrixCellEditor key={`${model.matrix}:${model.x}:${model.y}`} matrix={model.matrix} x={model.x} y={model.y} context={matrixEditor} depth={depth} />);
    case "none":
    // The inspector shows the selection only: a hover is never pinned, and the status line names it.
    case "eventHover":
    case "tile":
      return (
        <div className={styles.panel}>
          <EmptyState what="Nichts ausgewählt." why="Wähle links einen Ort oder klicke in der Schau einen Marker an." />
        </div>
      );
  }
}
