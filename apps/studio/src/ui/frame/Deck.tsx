import { memo, useSyncExternalStore } from "react";
import type { FunctionComponent, MutableRefObject, ReactNode } from "react";
import { DockviewDefaultTab, DockviewReact } from "dockview-react";
import type { DockviewApi, DockviewReadyEvent, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import { deckPanels, dropLayout, isSaneLayout, layoutKey, loadLayout, saveLayout } from "../../workbench/layout";
import type { DeckId, PanelSpec } from "../../workbench/layout";
import { scriptDirtyStore } from "../../workbench/script/scriptDirtyStore";
import { useShell } from "./ShellContext";
import { HAND_DEFAULT } from "./shellState";
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
  /**
   * The deck where wizards show (the world's): it keeps a slot beside the hand's dock for the wizard's step (`data-wizard-hand`, filled
   * through a portal). True while a wizard is open: the slot shows in place of the dock. Undefined: no slot.
   */
  wizard?: boolean;
};

/**
 * Panels cannot be closed (there is no way to bring one back), only moved. The tab "Script" carries the cobalt dot while a script is unsaved;
 * its title stays "Script".
 */
function PanelTab(props: IDockviewPanelHeaderProps) {
  const unsavedScripts = useSyncExternalStore(scriptDirtyStore.subscribe, scriptDirtyStore.get);
  return (
    <span className={styles.tab}>
      <DockviewDefaultTab {...props} hideClose />
      {props.api.id === "script" && unsavedScripts > 0 && <span className={styles.dot} role="img" aria-label="ungespeichert" />}
    </span>
  );
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
      initialWidth: p.width,
    });
  }
  // The tabs were added into the anchor's group and took focus in turn; the anchor is what opens.
  const anchor = panels[0];
  if (anchor) api.getPanel(anchor.id)?.api.setActive();
}

/**
 * Puts both docks of `area` back to their default layouts and remembers them. A dock that is not built yet (the world's, reset from an
 * area without a deck before the world was shown) forgets its stored layout, so it starts from the default when it is built.
 */
export function resetLayout(area: AreaId, docks: Record<DeckId, MutableRefObject<DockviewApi | null>>): void {
  for (const deck of ["schau", "hand"] as const) {
    const api = docks[deck].current;
    if (!api) {
      dropLayout(layoutKey(area, deck));
      continue;
    }
    applyDefaultLayout(api, deckPanels(area, deck));
    syncTabBars(api);
    saveLayout(layoutKey(area, deck), api.toJSON());
  }
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
 * Each dock is its own dockview with its own layout key, so a panel moves within a dock, never between the two. Every area with a deck has
 * its own `Deck` (the shell keeps them all mounted); the hand's height and whether it is collapsed are the area's own. While a wizard is open
 * (world only), its step takes the hand's place.
 */
export function Deck({ area, schauComponents, handComponents, schauRef, handRef, hinge, wizard }: DeckProps) {
  const { state } = useShell();
  const collapsed = state.handCollapsed[area];
  // A wizard needs room: a collapsed hand is lifted to at least the default height while it is open (the remembered height stays).
  const wizardHeight = collapsed ? Math.max(state.handHeight[area], HAND_DEFAULT) : state.handHeight[area];
  return (
    <div className={styles.deck}>
      <div className={styles.schau} id={`dock-${area}-schau`} data-dock="schau">
        <DeckDock area={area} deck="schau" components={schauComponents} apiRef={schauRef} />
      </div>
      {hinge}
      {/* Behind an open wizard the dock stays mounted (CodeMirror and the scene keep their state), only hidden. */}
      <div className={styles.hand} id={`dock-${area}-hand`} data-dock="hand" hidden={collapsed || wizard === true} style={{ height: state.handHeight[area] } /* dyn */}>
        <DeckDock area={area} deck="hand" components={handComponents} apiRef={handRef} />
      </div>
      {wizard !== undefined && <div className={styles.hand} data-wizard-hand="" hidden={!wizard} style={{ height: wizardHeight } /* dyn */} />}
    </div>
  );
}
