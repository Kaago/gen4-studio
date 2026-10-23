import { memo } from "react";
import type { MutableRefObject } from "react";
import { DockviewDefaultTab, DockviewReact } from "dockview-react";
import type { DockviewApi, DockviewReadyEvent, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import { DEFAULT_PANELS, isSaneLayout, loadLayout, saveLayout } from "./layout";
import { HeadersPanel, InspectorPanel, MatrixTab, ProblemsTab, ScriptTab, StatusPanel, TextTab, WorldMapTab, WorldPanel } from "./panels";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData, WorkbenchLive } from "./WorkbenchContext";

/** Must stay a module-level constant: dockview would remount every panel if this object changed. */
const COMPONENTS: Record<string, React.FunctionComponent<IDockviewPanelProps>> = {
  headers: HeadersPanel,
  world: WorldPanel,
  matrix: MatrixTab,
  worldmap: WorldMapTab,
  text: TextTab,
  script: ScriptTab,
  problems: ProblemsTab,
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
  // The raster, world map, text and script tabs were added into the world's group and took focus; the scene is what opens.
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

/** The dock, memoised: the providers above it change with every hover, the dock itself must not. */
const Dock = memo(function Dock({ apiRef }: { apiRef: MutableRefObject<DockviewApi | null> }) {
  function onReady(event: DockviewReadyEvent) {
    apiRef.current = event.api;
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
    <div style={{ flex: 1, minHeight: 0 }}>
      <DockviewReact components={COMPONENTS} onReady={onReady} className="dockview-theme-g4" defaultTabComponent={PanelTab} />
    </div>
  );
});

/** `apiRef` hands App the dockview api (focusing a panel when a link leads there). `live` is the volatile part (hover, message). */
export default function Workbench({ data, live, apiRef }: { data: WorkbenchData; live: WorkbenchLive; apiRef: MutableRefObject<DockviewApi | null> }) {
  return (
    <WorkbenchProviders data={data} live={live}>
      <Dock apiRef={apiRef} />
    </WorkbenchProviders>
  );
}
