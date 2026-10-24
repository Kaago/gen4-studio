import { Suspense, lazy, useCallback, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { ProjectInfo } from "../api";
import { useStableCallback } from "../app/useStable";
import { useBusRevision } from "../bus/useBusEvent";
import { ItemsArea, ItemsAreaList, ItemsHand, ItemsStatus, ItemsView } from "../data/ItemsArea";
import { PokemonArea, PokemonHand, PokemonList, PokemonStatus, PokemonView } from "../data/PokemonArea";
import type { DataArea } from "../data/dataModel";
import type { DataStore } from "../data/dataStore";
import type { DataAreaProps } from "../data/useDataArea";
import type { DockComponents } from "../ui/frame/Deck";
import { useShell } from "../ui/frame/ShellContext";
import { EmptyState } from "../ui/primitives";
import { lockedText } from "../world/events";
import MatrixScene from "../world/MatrixScene";
import { thumbnails } from "../world/thumbnailStore";
import { ContextPanel } from "./ContextPanel";
import { Inspector } from "./Inspector";
import type { Crumb } from "./breadcrumb";
import type { DeckArea, DeckId } from "./layout";
import type { DataNav } from "./navigation";
import { Legend } from "./Legend";
import { MatrixPanel } from "./MatrixPanel";
import type { MatrixPanelContext } from "./MatrixPanel";
import { SceneBar } from "./SceneBar";
import { ProblemsPanel } from "./problems/ProblemsPanel";
import type { ProblemsPanelContext } from "./problems/ProblemsPanel";
import type { ScriptPanelContext } from "./script/ScriptPanel";
import { TextArea, TextHints, TextStatus } from "./text/TextArea";
import type { TextAreaProps, TextShown } from "./text/TextArea";
import { TextHand } from "./text/TextHand";
import { TextList } from "./text/TextList";
import { TextView } from "./text/TextView";
import styles from "./panels.module.css";
import { useEvents } from "./useEvents";
import { useThumbnailReset, useThumbnailToolReport } from "./useThumbnailReset";
import { ToolsPanel } from "./ToolsPanel";
import { WorldExplorer } from "./WorldExplorer";
import { WorldMapPanel } from "./WorldMapPanel";
import type { WorldMapContext } from "./WorldMapPanel";
import { useInspectorModel, useMatrixSelection, useWorkbench } from "./WorkbenchContext";

/** The script panel pulls in CodeMirror and the language client: its chunk loads on its own, not with the app. */
const ScriptPanel = lazy(() => import("./script/ScriptPanel"));

/** The dockview panels below. Each reads only the contexts it renders (see `WorkbenchLive`). */

/** The explorer: no dock panel but the shell's list column; whether it is folded is the shell's state of the world area. */
export function ExplorerPane() {
  const d = useWorkbench();
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.welt;
  const onToggleList = useStableCallback(() => setListOpen("welt", !listOpen));
  // Counts the links that lead to a header: the list scrolls to the selection again, also when it was selected already.
  const revealToken = useBusRevision(d.bus, "headerReveal");
  // A link to a script also gives the script panel the focus (`onOpenScript` alone only opens the document).
  const goTo = d.goTo;
  const onOpenScript = useCallback((file: number, line?: number) => goTo({ kind: "script", file, line }), [goTo]);
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
      depth={d.depth}
      onSelect={d.onSelect}
      onOpenWizard={d.onOpenWizard}
      scriptsAdopted={d.scripts.status?.adopted === true}
      onOpenScript={onOpenScript}
      listOpen={listOpen}
      onToggleList={onToggleList}
    />
  );
}

export function WorldPanel() {
  const d = useWorkbench();
  const first = d.headers[0];
  if (!d.matrix) {
    return (
      <div className={styles.empty}>
        <EmptyState
          what="Noch kein Ort gewählt."
          why="Wähle links einen Ort, dann zeigt die Schau seine Karten."
          action={first ? { label: "Ersten Ort zeigen", onClick: () => d.onSelect(first) } : undefined}
        />
      </div>
    );
  }
  return (
    <div className={styles.stage}>
      <MatrixScene
        matrix={d.matrix}
        areaDataId={d.areaDataId}
        showPermissions={d.showPermissions}
        eventFileId={d.eventFileId}
        showEvents={d.showEvents}
        headers={d.headers}
        paintEnabled={d.paintEnabled}
        tool={d.tool}
        onTileClick={d.onTileClick}
        onBuildings={d.onBuildings}
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
      <SceneBar
        showPermissions={d.showPermissions}
        showEvents={d.showEvents}
        onShowPermissions={d.onShowPermissions}
        onShowEvents={d.onShowEvents}
      />
      <Legend />
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
  // A missing apicula shows once in the status strip (spec §4.4), not as a placeholder tooltip alone.
  useThumbnailToolReport(thumbnails, onError);
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
      depth: d.depth,
      goTo,
      onEdited: onMatrixEdited,
      onCountsChanged,
      onError,
    }),
    [info, headers, areaDataId, memberCounts, sceneMatrix, matrixRevision, selection, d.depth, goTo, onMatrixEdited, onCountsChanged, onError],
  );
  return <MatrixPanel context={context} />;
}

export function WorldMapTab() {
  const d = useWorkbench();
  // Changes whenever matrix data may have moved behind the panel (edit, undo, redo, reload).
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const { headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, goTo, onError, onLoadStatus } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<WorldMapContext>(
    () => ({ headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, revision: matrixRevision, goTo, onError, onLoadStatus }),
    [headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, matrixRevision, goTo, onError, onLoadStatus],
  );
  return <WorldMapPanel context={context} />;
}

export function ScriptTab() {
  const d = useWorkbench();
  const { info, scripts, problems, bus, depth, onError } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes (`scripts` with its revision).
  const context = useMemo<ScriptPanelContext>(() => ({ project: info, scripts, problems, bus, depth, onError }), [info, scripts, problems, bus, depth, onError]);
  return (
    <Suspense fallback={<p className={styles.loading}>Editor wird geladen …</p>}>
      <ScriptPanel context={context} />
    </Suspense>
  );
}

export function ProblemsTab() {
  const { problems, depth, goTo } = useWorkbench();
  const context = useMemo<ProblemsPanelContext>(() => ({ problems, depth, goTo }), [problems, depth, goTo]);
  return <ProblemsPanel context={context} />;
}

export function ToolsTab() {
  const { tool, onTool, eventFileId, eventLock } = useWorkbench();
  const { density } = useShell();
  // The four event tools wait for a shown place and an event file that may be written.
  const eventsBlocked = eventLock ? lockedText(eventLock.reason) : eventFileId === null ? "Erst einen Ort wählen." : null;
  return <ToolsPanel tool={tool} onTool={onTool} eventsBlocked={eventsBlocked} density={density} />;
}

export function ContextTab() {
  const d = useWorkbench();
  // The events of the shown place, read again when events moved (an edit, a drag, undo and redo). The scene and the inspector report a
  // failed read already; here it only ends the "Lade …".
  const eventsRevision = useBusRevision(d.bus, ["eventData", "eventsRefetch"]);
  const [failedFile, setFailedFile] = useState<number | null>(null);
  const events = useEvents(d.eventFileId, () => setFailedFile(d.eventFileId), eventsRevision);
  const loading = d.eventFileId !== null && events === null && failedFile !== d.eventFileId;
  const { goTo, onSelectEvent, onSelectBuilding } = d;
  const onOpenHeader = useCallback((id: number) => goTo({ kind: "header", id }), [goTo]);
  return (
    <ContextPanel
      hasPlace={d.matrix !== null}
      events={events}
      headers={d.headers}
      buildings={d.buildings}
      selectedEvent={d.selectedEvent}
      selectedBuilding={d.selectedBuilding}
      onSelectEvent={onSelectEvent}
      onSelectBuilding={onSelectBuilding}
      onOpenHeader={onOpenHeader}
      loading={loading}
    />
  );
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
  // A link to a script also gives the script panel the focus (`onOpenScript` alone only opens the document).
  const goTo = d.goTo;
  const openScript = (file: number, line?: number) => goTo({ kind: "script", file, line });
  // Read when a queued form save gets its turn: a project opened since then is not the one the save was made in.
  const openProject = useRef(d.info);
  openProject.current = d.info;
  const isCurrentProject = useCallback((project: ProjectInfo | null) => openProject.current === project, []);
  return (
    <Inspector
      model={inspector}
      depth={d.depth}
      headerEditor={{
        headers: d.headers,
        counts: d.memberCounts,
        revision: headerRevision,
        onChanged: d.onHeaderChanged,
        goTo: d.goTo,
        eventRevision,
        onOpenWizard: d.onOpenWizard,
        onAddWarp: () => void d.onAddEvent("warp"),
        info: d.info,
        scriptsAdopted: d.scripts.status?.adopted === true,
        scriptsRevision: d.scripts.revision,
        onOpenScript: openScript,
        onLevelScriptSaved: d.scripts.onLevelScriptSaved,
        isCurrentProject,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
      areaEditor={{ counts: d.memberCounts, dirty: d.dirtyAssets, revision: areaRevision, onChanged: d.onAreaChanged, onError: d.onError }}
      buildingEditor={{
        areaDataId: d.areaDataId,
        counts: d.memberCounts,
        dirty: d.dirtyAssets,
        revision: buildingRevision,
        onChanged: d.onBuildingChanged,
        onAdd: d.onAddBuilding,
        onDelete: d.onDeleteBuilding,
        onError: d.onError,
      }}
      eventEditor={{
        headers: d.headers,
        dirty: d.dirtyAssets,
        family: d.info?.family ?? null,
        revision: eventRevision,
        onChanged: d.onEventChanged,
        onAdd: d.onAddEvent,
        onDelete: d.onDeleteEvent,
        goTo: d.goTo,
        selected: d.selected,
        scriptsAdopted: d.scripts.status?.adopted === true,
        onOpenScript: openScript,
        onAddScript: d.scripts.add,
        onError: d.onError,
      }}
      matrixEditor={{
        dirty: d.dirtyAssets,
        headerCount: d.headers.length,
        goTo: d.goTo,
        counts: d.memberCounts,
        revision: matrixRevision,
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
        onMapImported: d.onMapImported,
      }}
    />
  );
}

/**
 * The text area's model around the frame (App puts it around the shell): the texts' list, view, hand and status parts read it. The
 * context it is given is memoised, so the area renders again only when one of its inputs changes, not for a hover or a message.
 */
export function TextAreaHost({ onShown, children }: { onShown?: (shown: TextShown) => void; children?: ReactNode }) {
  const d = useWorkbench();
  // Changes whenever a text archive may have moved behind the area (undo, redo, reload), and after the area's own edits (it tells the
  // data areas through the bus too).
  const textRevision = useBusRevision(d.bus, "textData");
  const { info, headers, memberCounts, textTarget, problems, goTo, onEdited, onCountsChanged, onError, bus } = d;
  const context = useMemo<TextAreaProps>(
    () => ({ project: info, headers, counts: memberCounts, target: textTarget, revision: textRevision, problems, goTo, onEdited, bus, onCountsChanged, onError, onShown }),
    [info, headers, memberCounts, textTarget, textRevision, problems, goTo, onEdited, bus, onCountsChanged, onError, onShown],
  );
  return <TextArea context={context}>{children}</TextArea>;
}

export { TextHints, TextStatus };

/** The list of the texts: the text archives; whether it is folded is the shell's state of the texts. */
export function TextArchivesPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.texte;
  const onToggle = useStableCallback(() => setListOpen("texte", !listOpen));
  return <TextList open={listOpen} onToggle={onToggle} />;
}

/** The view of the texts: the chosen message on the DS screen. */
export function TextViewTab() {
  return <TextView />;
}

/** The messages of the chosen archive, the search and the field of the chosen message. */
export function TextEditTab() {
  return <TextHand />;
}

/**
 * The models of the two data areas around the frame (App puts them around the shell, inside the text area's): their lists, views, hands
 * and status parts read them. Each area reads its lists from the store the first time it shows; the contexts are memoised, so an area
 * renders again only when one of its inputs changes.
 */
export function DataAreaHost({ store, target, onTrail, children }: { store: DataStore; target: DataNav | null; onTrail: (area: DataArea, crumbs: Crumb[]) => void; children?: ReactNode }) {
  const { info, goTo, dirtyAssets: dirty, onEdited, bus, headers } = useWorkbench();
  const shown = useShell().state.area;
  const pokemonTrail = useCallback((crumbs: Crumb[]) => onTrail("pokemon", crumbs), [onTrail]);
  const itemsTrail = useCallback((crumbs: Crumb[]) => onTrail("gegenstaende", crumbs), [onTrail]);
  const pokemonActive = shown === "pokemon";
  const itemsActive = shown === "gegenstaende";
  const pokemon = useMemo<DataAreaProps>(
    () => ({ project: info, store, active: pokemonActive, target, goTo, onTrail: pokemonTrail, dirty, onEdited, bus, headers }),
    [info, store, pokemonActive, target, goTo, pokemonTrail, dirty, onEdited, bus, headers],
  );
  const items = useMemo<DataAreaProps>(
    () => ({ project: info, store, active: itemsActive, target, goTo, onTrail: itemsTrail, dirty, onEdited, bus, headers }),
    [info, store, itemsActive, target, goTo, itemsTrail, dirty, onEdited, bus, headers],
  );
  return (
    <PokemonArea context={pokemon}>
      <ItemsArea context={items}>{children}</ItemsArea>
    </PokemonArea>
  );
}

export { ItemsStatus, PokemonStatus };

/** The list of the Pokémon area; whether it is folded is the shell's state of the area. */
export function PokemonListPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.pokemon;
  const onToggle = useStableCallback(() => setListOpen("pokemon", !listOpen));
  return <PokemonList open={listOpen} onToggle={onToggle} />;
}

/** The list of the Gegenstände area. */
export function ItemsListPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.gegenstaende;
  const onToggle = useStableCallback(() => setListOpen("gegenstaende", !listOpen));
  return <ItemsAreaList open={listOpen} onToggle={onToggle} />;
}

/**
 * The panels of the view dock and of the hand dock of each area, by id. Module-level constants: dockview would remount every panel if they
 * changed. The problems are in the hands of Welt and Texte: each area's dock is its own dockview, and every instance reads the one problems
 * store.
 */
export const DECK_COMPONENTS: Record<DeckArea, Record<DeckId, DockComponents>> = {
  welt: {
    schau: { world: WorldPanel, matrix: MatrixTab, worldmap: WorldMapTab },
    hand: { tools: ToolsTab, inspector: InspectorPanel, script: ScriptTab, problems: ProblemsTab, context: ContextTab },
  },
  texte: {
    schau: { textView: TextViewTab },
    hand: { textEdit: TextEditTab, problems: ProblemsTab },
  },
  pokemon: {
    schau: { pokemonView: PokemonView },
    hand: { pokemonEdit: PokemonHand },
  },
  gegenstaende: {
    schau: { itemView: ItemsView },
    hand: { itemEdit: ItemsHand },
  },
};
