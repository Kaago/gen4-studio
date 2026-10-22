import { createContext, useContext } from "react";
import { DockviewDefaultTab, DockviewReact, themeLight } from "dockview-react";
import type { DockviewApi, DockviewReadyEvent, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import type { EventKind, HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "../api";
import { StatusBar } from "../Toolbar";
import type { Message } from "../Toolbar";
import type { BuildingRef } from "../world/buildingLayer";
import type { BuildingRefresh } from "../world/buildings";
import type { EventRef, EventRefresh } from "../world/events";
import MatrixScene from "../world/MatrixScene";
import { HeaderList } from "./HeaderList";
import { AddEvent } from "./inspector/AddEvent";
import { Inspector } from "./Inspector";
import { MatrixPanel } from "./MatrixPanel";
import type { InspectorModel } from "./Inspector";
import { DEFAULT_PANELS, isSaneLayout, loadLayout, saveLayout } from "./layout";

/** Everything the panels show. App owns the state; panels read it through this context. */
export interface WorkbenchData {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
  matrix: MatrixDto | null;
  /** Area data the scene's models are textured with (follows the shown header, set together with `matrix`). */
  areaDataId: number;
  /** Incremented when the textures of the shown area data changed: the scene reloads its models. */
  modelsToken: number;
  showPermissions: boolean;
  onShowPermissions: (show: boolean) => void;
  /** Event file of the selected header (null: none selected). */
  eventFileId: number | null;
  showEvents: boolean;
  onShowEvents: (show: boolean) => void;
  paintEnabled: boolean;
  reloadToken: number;
  onEdited: () => void;
  onError: (e: unknown) => void;
  /** Load progress / failure summary of the scene; null clears it (only if it is still the shown message). */
  onLoadStatus: (message: Message | null) => void;
  /** What the inspector shows: the scene's hover while the pointer is over a cell, else the selected header. */
  inspector: InspectorModel;
  /** The scene reports what is under the pointer (`none` when it left every cell and marker). */
  onInspect: (model: InspectorModel) => void;
  memberCounts: MemberCounts;
  /** Bumped when the header table changed behind the header editor's back (undo, redo, reload). */
  headerRevision: number;
  /** A header field was written: refresh list, scene and dirty count. */
  onHeaderChanged: () => void | Promise<void>;
  /** Changes whenever area data may have changed behind the area editor (undo, redo, reload). */
  areaRevision: number;
  /** An area data field was written: refresh the dirty count and, if the shown models' tileset changed, the scene. */
  onAreaChanged: () => void | Promise<void>;
  /** Changes whenever matrix data may have changed behind the matrix panel and the cell editor (edit, undo, redo, reload). */
  matrixRevision: number;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onMatrixEdited: (index: number) => void | Promise<void>;
  /** A map or matrix member was appended: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  /** Pins `model` in the inspector (until another header or cell is chosen). */
  onPin: (model: InspectorModel) => void;
  /** The building selected in the scene (it is the pinned inspector model); null: none. */
  selectedBuilding: BuildingRef | null;
  /** A click in the scene chose a building (null: none). */
  onSelectBuilding: (ref: BuildingRef | null) => void;
  /** A drag moved a building and the move is written. */
  onBuildingMoved: (map: number) => void;
  /** Changes whenever buildings may have changed behind the building editor (a drag, undo, redo). */
  buildingRevision: number;
  /** A new request makes the scene read one map's buildings again. */
  buildingRefresh: BuildingRefresh | null;
  /** The building editor wrote a field: refresh the scene and the dirty count. */
  onBuildingChanged: (map: number, retryModel: number | null) => void | Promise<void>;
  /** Appends a building on the last hovered tile (else in `fallbackMap`, when given) and selects it. */
  onAddBuilding: (fallbackMap: number | null) => void | Promise<void>;
  /** Asks first, then deletes the building. */
  onDeleteBuilding: (map: number, index: number) => void | Promise<void>;
  /** The event selected in the scene (it is the pinned inspector model); null: none. */
  selectedEvent: EventRef | null;
  /** A click in the scene chose an event marker (null: none). */
  onSelectEvent: (ref: EventRef | null) => void;
  /** A drag moved an event and the move is written. */
  onEventMoved: (eventFile: number) => void;
  /** A new request makes the scene read one event file's events again. */
  eventRefresh: EventRefresh | null;
  /** Changes whenever events may have changed behind the event editor (a drag, undo, redo, add, delete). */
  eventRevision: number;
  /** The event editor wrote a field: refresh the scene and the dirty count. */
  onEventChanged: (eventFile: number) => void | Promise<void>;
  /** Adds an event on the last hovered tile of the selected header's event file and selects it. */
  onAddEvent: (kind: EventKind) => void | Promise<void>;
  /** Asks first, then deletes the event. */
  onDeleteEvent: (eventFile: number, kind: EventKind, index: number) => void | Promise<void>;
  message: Message | null;
  onDismissMessage: () => void;
}

const WorkbenchContext = createContext<WorkbenchData | null>(null);

function useWorkbench(): WorkbenchData {
  const data = useContext(WorkbenchContext);
  if (!data) throw new Error("panel rendered outside of the workbench");
  return data;
}

const toggleStyle: React.CSSProperties = {
  color: "#e5e7eb",
  background: "rgba(0,0,0,0.5)",
  padding: "4px 8px",
  borderRadius: 4,
};

function HeadersPanel() {
  const d = useWorkbench();
  return <HeaderList info={d.info} headers={d.headers} selected={d.selected} onSelect={d.onSelect} />;
}

function WorldPanel() {
  const d = useWorkbench();
  return (
    <div style={{ position: "relative", width: "100%", height: "100%", minWidth: 0, fontFamily: "system-ui, sans-serif" }}>
      {d.matrix ? (
        <>
          <MatrixScene
            matrix={d.matrix}
            areaDataId={d.areaDataId}
            modelsToken={d.modelsToken}
            showPermissions={d.showPermissions}
            eventFileId={d.eventFileId}
            showEvents={d.showEvents}
            headers={d.headers}
            paintEnabled={d.paintEnabled}
            reloadToken={d.reloadToken}
            onEdited={d.onEdited}
            onError={d.onError}
            onInspect={d.onInspect}
            onStatus={d.onLoadStatus}
            selectedMap={d.selectedBuilding?.map ?? null}
            selectedBuilding={d.selectedBuilding?.index ?? null}
            onSelectBuilding={d.onSelectBuilding}
            onBuildingMoved={d.onBuildingMoved}
            buildingRefresh={d.buildingRefresh}
            selectedEvent={d.selectedEvent}
            onSelectEvent={d.onSelectEvent}
            onEventMoved={d.onEventMoved}
            eventRefresh={d.eventRefresh}
          />
          <div style={{ position: "absolute", top: 8, left: 8, display: "flex", gap: 8 }}>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showPermissions} onChange={(e) => d.onShowPermissions(e.target.checked)} /> Permissions
              anzeigen
            </label>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showEvents} onChange={(e) => d.onShowEvents(e.target.checked)} /> Events anzeigen
            </label>
            <button
              onClick={() => void d.onAddBuilding(null)}
              disabled={!d.paintEnabled}
              title="Legt ein Gebäude auf der Kachel an, über der die Maus zuletzt war"
              style={{ ...toggleStyle, border: "none", font: "inherit" }}
            >
              Neues Gebäude
            </button>
            <AddEvent disabled={!d.paintEnabled || d.eventFileId === null} onAdd={(kind) => void d.onAddEvent(kind)} style={toggleStyle} />
          </div>
        </>
      ) : (
        <p style={{ padding: 16 }}>Header wählen, um die Matrix anzuzeigen. Klick setzt Kollision, Shift+Klick löscht sie.</p>
      )}
    </div>
  );
}

function MatrixTab() {
  const d = useWorkbench();
  const selection = d.inspector.kind === "matrixCell" ? d.inspector : null;
  return (
    <MatrixPanel
      context={{
        info: d.info,
        sceneMatrix: d.matrix?.index ?? null,
        revision: d.matrixRevision,
        selection: selection && { matrix: selection.matrix, x: selection.x, y: selection.y },
        onSelectCell: (matrix, x, y) => d.onPin({ kind: "matrixCell", matrix, x, y }),
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
    />
  );
}

function InspectorPanel() {
  const d = useWorkbench();
  return (
    <Inspector
      model={d.inspector}
      headerEditor={{
        headers: d.headers,
        counts: d.memberCounts,
        revision: d.headerRevision,
        onChanged: d.onHeaderChanged,
        onOpenArea: (id) => d.onPin({ kind: "area", id }),
        onError: d.onError,
      }}
      areaEditor={{ counts: d.memberCounts, revision: d.areaRevision, onChanged: d.onAreaChanged, onError: d.onError }}
      buildingEditor={{
        areaDataId: d.areaDataId,
        counts: d.memberCounts,
        revision: d.buildingRevision,
        onChanged: d.onBuildingChanged,
        onAdd: d.onAddBuilding,
        onDelete: d.onDeleteBuilding,
        onError: d.onError,
      }}
      eventEditor={{
        headers: d.headers,
        family: d.info?.family ?? null,
        revision: d.eventRevision,
        onChanged: d.onEventChanged,
        onAdd: d.onAddEvent,
        onDelete: d.onDeleteEvent,
        onOpenHeader: d.onSelect,
        onError: d.onError,
      }}
      matrixEditor={{
        headerCount: d.headers.length,
        counts: d.memberCounts,
        revision: d.matrixRevision,
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
    />
  );
}

function StatusPanel() {
  const d = useWorkbench();
  return (
    <div style={{ height: "100%", overflowY: "auto" }}>
      <StatusBar message={d.message} onDismiss={d.onDismissMessage} />
    </div>
  );
}

/** Must stay a module-level constant: dockview would remount every panel if this object changed. */
const COMPONENTS: Record<string, React.FunctionComponent<IDockviewPanelProps>> = {
  headers: HeadersPanel,
  world: WorldPanel,
  matrix: MatrixTab,
  inspector: InspectorPanel,
  status: StatusPanel,
};

/** Panels cannot be closed (there is no way to bring one back), only moved. */
function PanelTab(props: IDockviewPanelHeaderProps) {
  return <DockviewDefaultTab {...props} hideClose />;
}

function applyDefaultLayout(api: DockviewApi) {
  api.clear();
  for (const p of DEFAULT_PANELS) {
    api.addPanel({
      id: p.id,
      component: p.id,
      title: p.title,
      // Keep the DOM (and the three.js canvas) alive when a tab is hidden or the panel is moved.
      renderer: "always",
      position: p.position.referencePanel ? { direction: p.position.direction, referencePanel: p.position.referencePanel } : undefined,
      initialWidth: p.position.direction === "left" || p.position.direction === "right" ? p.size : undefined,
      initialHeight: p.position.direction === "below" ? p.size : undefined,
    });
  }
  // The matrix tab was added last into the world's group and took focus; the scene is what opens.
  api.getPanel("world")?.api.setActive();
}

/**
 * The status panel is a strip without tab header, with a minimum height below dockview's 100 px default.
 * Its group is locked so no other panel can be dropped onto it (the hidden header would leave no way
 * to drag it out again); `isSaneLayout` guarantees the status panel is alone in its group.
 */
function fitStatusStrip(api: DockviewApi) {
  const status = api.getPanel("status");
  const size = DEFAULT_PANELS.find((p) => p.id === "status")?.size;
  if (!status || size === undefined || status.group.panels.length !== 1) return;
  status.group.header.hidden = true;
  status.group.locked = true;
  status.group.api.setConstraints({ minimumHeight: size });
  // Groups restored from JSON were clamped to the 100 px default, so the strip is always reset to its height.
  status.group.api.setSize({ height: size });
}

/** Restores the stored layout when it is complete, otherwise builds the default one. */
function initLayout(api: DockviewApi) {
  const saved = loadLayout();
  if (saved && isSaneLayout(saved)) {
    try {
      api.fromJSON(saved as SerializedDockview);
      return;
    } catch {
      /* damaged layout: fall through to the default */
    }
  }
  applyDefaultLayout(api);
}

export default function Workbench({ data }: { data: WorkbenchData }) {
  function onReady(event: DockviewReadyEvent) {
    initLayout(event.api);
    fitStatusStrip(event.api);
    // The subscription ends with the api when dockview unmounts, so it needs no cleanup of its own.
    event.api.onDidLayoutChange(() => {
      const json = event.api.toJSON();
      // Intermediate states (clear, half-built default) must not overwrite the user's layout.
      if (isSaneLayout(json)) saveLayout(json);
    });
  }

  return (
    <WorkbenchContext.Provider value={data}>
      <div style={{ flex: 1, minHeight: 0 }}>
        <DockviewReact components={COMPONENTS} onReady={onReady} theme={themeLight} defaultTabComponent={PanelTab} />
      </div>
    </WorkbenchContext.Provider>
  );
}
