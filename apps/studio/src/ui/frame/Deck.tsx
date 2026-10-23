import { memo } from "react";
import type { FunctionComponent, MutableRefObject, ReactNode } from "react";
import { DockviewDefaultTab, DockviewReact } from "dockview-react";
import type { DockviewApi, DockviewReadyEvent, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import { deckPanels, isSaneLayout, layoutKey, loadLayout, saveLayout } from "../../workbench/layout";
import type { DeckId, PanelSpec } from "../../workbench/layout";
import { useShell } from "./ShellContext";
import type { AreaId } from "./shellState";
import styles from "./Deck.module.css";

/** The panels of a dock by id. A module-level constant of the caller: dockview would remount every panel if this object changed. */
export type DockComponents = Record<string, FunctionComponent<IDockviewPanelProps>>;

export type DeckProps = {
  area: AreaId;
  schauComponents: DockComponents;
  handComponents: DockComponents;
  /** Where each dock hands out its dockview api (a link focuses the panel that shows its target). */
  schauRef: MutableRefObject<DockviewApi | null>;
  handRef: MutableRefObject<DockviewApi | null>;
  /** The bar between the two docks. */
  hinge: ReactNode;
};

/** Panels cannot be closed (there is no way to bring one back), only moved. */
function PanelTab(props: IDockviewPanelHeaderProps) {
  return <DockviewDefaultTab {...props} hideClose />;
}

function applyDefaultLayout(api: DockviewApi, panels: readonly PanelSpec[]) {
  api.clear();
  for (const p of panels) {
    api.addPanel({
      id: p.id,
      component: p.id,
      title: p.title,
      // Keep the DOM (and the three.js canvas) alive when a tab is hidden or the panel is moved.
      renderer: "always",
      position: p.position.referencePanel ? { direction: p.position.direction, referencePanel: p.position.referencePanel } : undefined,
    });
  }
  // The tabs were added into the anchor's group and took focus in turn; the anchor is what opens.
  const anchor = panels[0];
  if (anchor) api.getPanel(anchor.id)?.api.setActive();
}

/** Restores the stored layout of this dock when it is complete, otherwise builds the dock's default one. Touches no other dock. */
function initLayout(api: DockviewApi, key: string, panels: readonly PanelSpec[]) {
  const saved = loadLayout(key);
  if (saved && isSaneLayout(saved, panels)) {
    try {
      api.fromJSON(saved as SerializedDockview);
      return;
    } catch {
      /* damaged layout: fall through to the default */
    }
  }
  applyDefaultLayout(api, panels);
}

/** A group with one panel needs no tab bar (the panel shows its own head); from two panels it shows. */
function syncTabBars(api: DockviewApi) {
  for (const group of api.groups) group.header.hidden = group.panels.length < 2;
}

/** One dock, memoised: the providers above it change with every hover, the dock itself must not. */
const DeckDock = memo(function DeckDock({ area, deck, components, apiRef }: { area: AreaId; deck: DeckId; components: DockComponents; apiRef: MutableRefObject<DockviewApi | null> }) {
  function onReady(event: DockviewReadyEvent) {
    const api = event.api;
    const key = layoutKey(area, deck);
    const panels = deckPanels(area, deck);
    apiRef.current = api;
    initLayout(api, key, panels);
    syncTabBars(api);
    // Intermediate states (clear, half-built default) must not overwrite the user's layout.
    const remember = () => {
      const json = api.toJSON();
      if (isSaneLayout(json, panels)) saveLayout(key, json);
    };
    remember();
    // The subscriptions end with the api when dockview unmounts, so they need no cleanup of their own.
    api.onDidAddPanel(() => syncTabBars(api));
    api.onDidRemovePanel(() => syncTabBars(api));
    api.onDidLayoutChange(() => {
      syncTabBars(api);
      remember();
    });
  }
  return (
    <div className={styles.dock}>
      <DockviewReact components={components} onReady={onReady} className="dockview-theme-g4" defaultTabComponent={PanelTab} />
    </div>
  );
});

/**
 * The deck of an area: the view (Schau) on top, the hinge, the hand below. The hand has the remembered height, the view the rest; a
 * collapsed hand is hidden (not unmounted: CodeMirror and the scene keep their state) and only the hinge is left.
 * Each dock is its own dockview with its own layout key, so a panel moves within a dock, never between the two.
 */
export function Deck({ area, schauComponents, handComponents, schauRef, handRef, hinge }: DeckProps) {
  const { state } = useShell();
  return (
    <div className={styles.deck}>
      <div className={styles.schau} id="dock-schau">
        <DeckDock area={area} deck="schau" components={schauComponents} apiRef={schauRef} />
      </div>
      {hinge}
      <div className={styles.hand} id="dock-hand" hidden={state.handCollapsed[area]} style={{ height: state.handHeight[area] } /* dyn */}>
        <DeckDock area={area} deck="hand" components={handComponents} apiRef={handRef} />
      </div>
    </div>
  );
}
