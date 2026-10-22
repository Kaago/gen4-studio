import { useMemo } from "react";
import { StatusBar } from "../Toolbar";
import { useBusRevision } from "../bus/useBusEvent";
import { lockedText } from "../world/events";
import MatrixScene from "../world/MatrixScene";
import { thumbnails } from "../world/thumbnailStore";
import { AddEvent } from "./inspector/AddEvent";
import { Inspector } from "./Inspector";
import { MatrixPanel } from "./MatrixPanel";
import type { MatrixPanelContext } from "./MatrixPanel";
import { useThumbnailReset } from "./useThumbnailReset";
import { WorldExplorer } from "./WorldExplorer";
import { useInspectorModel, useMatrixSelection, useMessage, useWorkbench } from "./WorkbenchContext";

const toggleStyle: React.CSSProperties = {
  color: "#e5e7eb",
  background: "rgba(0,0,0,0.5)",
  padding: "4px 8px",
  borderRadius: 4,
  fontSize: 12,
  whiteSpace: "nowrap",
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
};

/** The dockview panels below. Each reads only the contexts it renders (see `WorkbenchLive`). */

export function HeadersPanel() {
  const d = useWorkbench();
  // Counts the links that lead to a header: the list scrolls to the selection again, also when it was selected already.
  const revealToken = useBusRevision(d.bus, "headerReveal");
  return (
    <WorldExplorer
      info={d.info}
      headers={d.headers}
      matrices={d.matrices}
      matricesLoaded={d.matricesLoaded}
      matricesFailed={d.matricesFailed}
      selected={d.selected}
      shownMatrix={d.matrix?.index ?? null}
      revealToken={revealToken}
      onSelect={d.onSelect}
      onOpenWizard={d.onOpenWizard}
    />
  );
}

export function WorldPanel() {
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
            history={d.history}
            bus={d.bus}
            onEdited={d.onEdited}
            onError={d.onError}
            onInspect={d.onInspect}
            onStatus={d.onLoadStatus}
            selectedMap={d.selectedBuilding?.map ?? null}
            selectedBuilding={d.selectedBuilding?.index ?? null}
            onSelectBuilding={d.onSelectBuilding}
            onBuildingMoved={d.onBuildingMoved}
            selectedEvent={d.selectedEvent}
            onSelectEvent={d.onSelectEvent}
            onEventMoved={d.onEventMoved}
            onEventsLocked={d.onEventsLocked}
          />
          <div style={{ position: "absolute", top: 8, left: 8, right: 8, display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8 }}>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showPermissions} onChange={(e) => d.onShowPermissions(e.target.checked)} /> Permissions anzeigen
            </label>
            <label style={toggleStyle}>
              <input type="checkbox" checked={d.showEvents} onChange={(e) => d.onShowEvents(e.target.checked)} /> Events anzeigen
            </label>
            <button
              onClick={() => void d.onAddBuilding(null)}
              disabled={!d.paintEnabled}
              title="Legt ein Gebäude auf der Kachel an, über der die Maus zuletzt war"
              style={{ ...toggleStyle, border: "none", fontFamily: "inherit" }}
            >
              Neues Gebäude
            </button>
            <AddEvent
              disabled={!d.paintEnabled || d.eventFileId === null || d.eventLock !== null}
              title={d.eventLock ? lockedText(d.eventLock.reason) : undefined}
              onAdd={(kind) => void d.onAddEvent(kind)}
              style={toggleStyle}
            />
          </div>
        </>
      ) : (
        <p style={{ padding: 16 }}>Header wählen, um die Matrix anzuzeigen. Klick setzt Kollision, Shift+Klick löscht sie.</p>
      )}
    </div>
  );
}

export function MatrixTab() {
  const d = useWorkbench();
  // The hover is not in this context, only the selected cell is (the same object while the same cell shows).
  const selection = useMatrixSelection();
  // Changes whenever matrix data may have moved behind the panel (edit, undo, redo, reload).
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const info = d.info;
  const sceneMatrix = d.matrix?.index ?? null;
  const { goTo, onMatrixEdited, onCountsChanged, onError, headers, areaDataId, memberCounts, bus } = d;
  // The pictures of the grid come from the app-wide cache; it must not show another project's or tileset's.
  useThumbnailReset(bus, info?.root ?? null, thumbnails);
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<MatrixPanelContext>(
    () => ({
      info,
      headers,
      areaDataId,
      counts: memberCounts,
      sceneMatrix,
      revision: matrixRevision,
      selection,
      goTo,
      onEdited: onMatrixEdited,
      onCountsChanged,
      onError,
    }),
    [info, headers, areaDataId, memberCounts, sceneMatrix, matrixRevision, selection, goTo, onMatrixEdited, onCountsChanged, onError],
  );
  return <MatrixPanel context={context} />;
}

export function InspectorPanel() {
  const d = useWorkbench();
  const inspector = useInspectorModel();
  // Each editor reads its asset again when its revision changes: data moved behind its back (a drag, undo, redo, add, delete, reload).
  const headerRevision = useBusRevision(d.bus, "headerTable");
  const areaRevision = useBusRevision(d.bus, "areaData");
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const buildingRevision = useBusRevision(d.bus, "buildingData");
  const eventRevision = useBusRevision(d.bus, "eventData");
  return (
    <Inspector
      model={inspector}
      headerEditor={{
        headers: d.headers,
        counts: d.memberCounts,
        revision: headerRevision,
        onChanged: d.onHeaderChanged,
        goTo: d.goTo,
        eventRevision,
        onOpenWizard: d.onOpenWizard,
        onAddWarp: () => void d.onAddEvent("warp"),
        onError: d.onError,
      }}
      areaEditor={{ counts: d.memberCounts, revision: areaRevision, onChanged: d.onAreaChanged, onError: d.onError }}
      buildingEditor={{
        areaDataId: d.areaDataId,
        counts: d.memberCounts,
        revision: buildingRevision,
        onChanged: d.onBuildingChanged,
        onAdd: d.onAddBuilding,
        onDelete: d.onDeleteBuilding,
        onError: d.onError,
      }}
      eventEditor={{
        headers: d.headers,
        family: d.info?.family ?? null,
        revision: eventRevision,
        onChanged: d.onEventChanged,
        onAdd: d.onAddEvent,
        onDelete: d.onDeleteEvent,
        goTo: d.goTo,
        onError: d.onError,
      }}
      matrixEditor={{
        headerCount: d.headers.length,
        goTo: d.goTo,
        counts: d.memberCounts,
        revision: matrixRevision,
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
    />
  );
}

export function StatusPanel() {
  const d = useWorkbench();
  const message = useMessage();
  return (
    <div style={{ height: "100%", overflowY: "auto" }}>
      <StatusBar message={message} onDismiss={d.onDismissMessage} />
    </div>
  );
}
