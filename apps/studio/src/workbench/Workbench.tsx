import { createContext, useContext } from "react";
import { DockviewDefaultTab, DockviewReact, themeLight } from "dockview-react";
import type { DockviewApi, DockviewReadyEvent, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import type { HeaderSummary, MatrixDto, ProjectInfo } from "../api";
import { StatusBar } from "../Toolbar";
import type { Message } from "../Toolbar";
import MatrixScene from "../world/MatrixScene";
import { HeaderList } from "./HeaderList";
import { Inspector } from "./Inspector";
import type { InspectorModel } from "./Inspector";
import { DEFAULT_PANELS, isSaneLayout, loadLayout, saveLayout } from "./layout";

/** Everything the panels show. App owns the state; panels read it through this context. */
export interface WorkbenchData {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
  matrix: MatrixDto | null;
  areaDataId: number;
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
  inspector: InspectorModel;
  setInspector: (model: InspectorModel) => void;
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
            showPermissions={d.showPermissions}
            eventFileId={d.eventFileId}
            showEvents={d.showEvents}
            headers={d.headers}
            paintEnabled={d.paintEnabled}
            reloadToken={d.reloadToken}
            onEdited={d.onEdited}
            onError={d.onError}
            onInspect={d.setInspector}
            onStatus={d.onLoadStatus}
          />
          <div style={{ position: "absolute", top: 8, left: 8, display: "flex", gap: 8 }}>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showPermissions} onChange={(e) => d.onShowPermissions(e.target.checked)} /> Permissions
              anzeigen
            </label>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showEvents} onChange={(e) => d.onShowEvents(e.target.checked)} /> Events anzeigen
            </label>
          </div>
        </>
      ) : (
        <p style={{ padding: 16 }}>Header wählen, um die Matrix anzuzeigen. Klick setzt Kollision, Shift+Klick löscht sie.</p>
      )}
    </div>
  );
}

function InspectorPanel() {
  return <Inspector model={useWorkbench().inspector} />;
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
