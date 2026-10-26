import { memo, useMemo, useSyncExternalStore } from "react";
import type { FunctionComponent, KeyboardEvent, MutableRefObject, PointerEvent, ReactNode } from "react";
import { DockviewDefaultTab, DockviewReact } from "dockview-react";
import type { DockviewApi, DockviewGroupPanel, DockviewReadyEvent, DockviewTheme, GetTabContextMenuItemsParams, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import { AREA_PANELS, DEFAULT_ARRANGEMENT, dropLayout, isSaneLayout, layoutKey, loadArrangement, loadLayout, saveArrangement, saveLayout } from "../../workbench/layout";
import type { Arrangement, DeckArea } from "../../workbench/layout";
import { scriptDirtyStore } from "../../workbench/script/scriptDirtyStore";
import { messages } from "./dockMessages";
import { tabMenuItems } from "./dockMenu";
import { workMenuByKeyboard } from "./dockMenuKeys";
import { applyArrangement, dockPanel, fillWhileMaximized, floatPanel, isSchauGroup, keepViewGroupClosed, keepViewGroupInPlace, keepViewPanelsInPlace, restoreLimits } from "./dockRules";
import styles from "./Deck.module.css";

/**
 * The dock's theme (`ui/tokens/dockview-theme.css`). Given as dockview's theme, not as the class of its grid: dockview puts the class on its
 * shell, which also holds the floating groups and the tab menus (beside the grid); otherwise they would keep dockview's own dark theme.
 * No tab group lines, as with dockview's default.
 */
const DOCK_THEME: DockviewTheme = { name: "g4", className: "dockview-theme-g4", tabGroupIndicator: "none" };

/** The panels of a dock by id. A module-level constant of the caller: dockview would remount every panel if this object changed. */
export type DockComponents = Record<string, FunctionComponent<IDockviewPanelProps>>;

export type DeckProps = {
  area: DeckArea;
  components: DockComponents;
  /** Where the dock hands out its dockview api (a link focuses the panel that shows its target). */
  dockRef: MutableRefObject<DockviewApi | null>;
  /** The bar above the dock. */
  hinge: ReactNode;
  /**
   * The deck where wizards show (the world's): it keeps a strip under the dock for the wizard's step (`data-wizard-hand`, filled through a
   * portal). True while a wizard is open: the strip shows and the dock above it shrinks. Undefined: no strip.
   */
  wizard?: boolean;
};

/**
 * Panels cannot be closed (there is no way to bring one back), only moved. A double click maximizes the tab's group or restores it (only a
 * group in the grid can be maximized). The tab "Script" carries the cobalt dot while a script is unsaved; its title stays "Script".
 */
function PanelTab(props: IDockviewPanelHeaderProps) {
  const unsavedScripts = useSyncExternalStore(scriptDirtyStore.subscribe, scriptDirtyStore.get);
  const onDoubleClick = () => {
    if (props.api.isMaximized()) props.api.exitMaximized();
    else if (props.api.location.type === "grid") props.api.maximize();
  };
  // `data-dock-tab` names the panel: the F6 round finds the content of a group's active panel by it (dockview renders it over the group).
  return (
    <span className={styles.tab} data-dock-tab={props.api.id} onDoubleClick={onDoubleClick}>
      <DockviewDefaultTab {...props} hideClose />
      {props.api.id === "script" && unsavedScripts > 0 && <span className={styles.dot} role="img" aria-label="ungespeichert" />}
    </span>
  );
}

/** The panel components wrapped once per components object, so dockview gets the same components on every render. */
const wrapped = new WeakMap<DockComponents, DockComponents>();

/** `Component` inside `div[data-dock-panel="<id>"]`, as high as the panel: the gate finds each panel by it. */
function withPanelAttr(id: string, Component: FunctionComponent<IDockviewPanelProps>): FunctionComponent<IDockviewPanelProps> {
  function DockPanel(props: IDockviewPanelProps) {
    return (
      <div className={styles.panel} data-dock-panel={id}>
        <Component {...props} />
      </div>
    );
  }
  DockPanel.displayName = `DockPanel(${id})`;
  return DockPanel;
}

function withPanelAttrs(components: DockComponents): DockComponents {
  let result = wrapped.get(components);
  if (!result) {
    result = Object.fromEntries(Object.entries(components).map(([id, Component]) => [id, withPanelAttr(id, Component)]));
    wrapped.set(components, result);
  }
  return result;
}

/** Marks a group's element with its id, and the view group as such (`data-dock-group-kind="schau"`). */
function markGroup(group: DockviewGroupPanel) {
  group.element.setAttribute("data-dock-group", group.id);
  if (isSchauGroup(group)) group.element.setAttribute("data-dock-group-kind", "schau");
  else group.element.removeAttribute("data-dock-group-kind");
}

/** Builds `arrangement` in the dock and remembers it as the area's arrangement and layout. */
function arrange(api: DockviewApi, area: DeckArea, arrangement: Arrangement) {
  applyArrangement(api, area, arrangement);
  api.groups.forEach(markGroup);
  saveArrangement(area, arrangement);
  saveLayout(layoutKey(area), api.toJSON());
}

/**
 * Builds `arrangement` (by default the area's default) in the dock of `area` and remembers it. A dock that is not built yet (the world's,
 * reset from an area without a deck before the world was shown) forgets its stored layout and arrangement, so it starts from them when it
 * is built.
 */
export function resetLayout(area: DeckArea, dockRef: MutableRefObject<DockviewApi | null>, arrangement: Arrangement = DEFAULT_ARRANGEMENT[area]): void {
  const api = dockRef.current;
  if (!api) {
    dropLayout(layoutKey(area));
    saveArrangement(area, arrangement);
    return;
  }
  arrange(api, area, arrangement);
}

/**
 * Restores the stored layout of the dock when it is complete (and no view panel floats in it), otherwise builds the area's arrangement.
 * The width limits of a group are not stored: a panel whose step has them gets them again when it sits alone in a docked group
 * (`restoreLimits`).
 */
function initLayout(api: DockviewApi, area: DeckArea) {
  const saved = loadLayout(layoutKey(area));
  if (saved && isSaneLayout(saved, AREA_PANELS[area])) {
    try {
      api.fromJSON(saved as SerializedDockview);
      restoreLimits(api, area);
      return;
    } catch {
      /* damaged layout: fall through to the arrangement */
    }
  }
  // The first build (or one after an unusable layout) remembers its arrangement, as a chosen one is.
  const arrangement = loadArrangement(area);
  applyArrangement(api, area, arrangement);
  saveArrangement(area, arrangement);
}

/** The dock, memoised: the providers above it change with every hover, the dock itself must not. */
const DeckDock = memo(function DeckDock({ area, components, apiRef }: { area: DeckArea; components: DockComponents; apiRef: MutableRefObject<DockviewApi | null> }) {
  const panels = useMemo(() => withPanelAttrs(components), [components]);
  // The tab menu works on this dock; it is asked for its items on every right click.
  const menuItems = useMemo(
    () => (params: GetTabContextMenuItemsParams) =>
      tabMenuItems(params, {
        float: (id) => void floatPanel(params.api, id),
        dock: (id) => void dockPanel(params.api, area, id),
        reset: () => resetLayout(area, apiRef, loadArrangement(area)),
      }),
    [area, apiRef],
  );
  function onReady(event: DockviewReadyEvent) {
    const api = event.api;
    apiRef.current = api;
    initLayout(api, area);
    api.groups.forEach(markGroup);
    // Intermediate states (clear, half-built arrangement) must not overwrite the user's layout.
    const remember = () => {
      const json = api.toJSON();
      if (isSaneLayout(json, AREA_PANELS[area])) saveLayout(layoutKey(area), json);
    };
    remember();
    // The subscriptions end with the api when dockview unmounts, so they need no cleanup of their own.
    api.onDidAddGroup(markGroup);
    api.onWillDragPanel(keepViewPanelsInPlace);
    api.onWillDragGroup(keepViewGroupInPlace);
    api.onWillShowOverlay(keepViewGroupClosed);
    api.onDidMaximizedGroupChange(fillWhileMaximized(api, area));
    api.onDidLayoutChange(() => {
      api.groups.forEach(markGroup);
      remember();
    });
  }
  return (
    <DockviewReact
      components={panels}
      onReady={onReady}
      theme={DOCK_THEME}
      defaultTabComponent={PanelTab}
      floatingGroupBounds="boundedWithinViewport"
      floatingGroupDragHandle="titlebar"
      messages={messages}
      getTabContextMenuItems={menuItems}
    />
  );
});

/**
 * Shift+F10 on a tab opens its context menu, as a right click does (dockview listens for the contextmenu event on the tab), and lets the
 * keyboard work it.
 */
function openTabMenu(event: KeyboardEvent<HTMLElement>) {
  if (event.key !== "F10" || !event.shiftKey) return;
  const tab = (event.target as HTMLElement).closest<HTMLElement>('[role="tab"]');
  if (!tab || !event.currentTarget.contains(tab)) return;
  event.preventDefault();
  const box = tab.getBoundingClientRect();
  tab.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: box.left, clientY: box.bottom }));
  // dockview builds the menu for the mouse; opened by key, it is worked by keys. The focus comes back to the panel's tab, found anew
  // (an item may have moved the panel into another group).
  const menu = document.querySelector<HTMLElement>(".dv-context-menu");
  const panel = tab.querySelector<HTMLElement>("[data-dock-tab]")?.dataset.dockTab;
  const dock = event.currentTarget;
  if (!menu || panel === undefined) return;
  workMenuByKeyboard(menu, () => dock.querySelector<HTMLElement>(`[data-dock-tab="${CSS.escape(panel)}"]`)?.closest<HTMLElement>('[role="tab"]')?.focus());
}

/**
 * Shift+pointerdown on a tab floats it, on a group's empty header the whole group: dockview does that itself, without asking
 * `onWillDragPanel`. On the view group's header (its tabs and its empty part) it is cancelled before dockview sees it (dockview leaves an
 * event alone whose default is prevented): the view stays docked. Elsewhere the gesture stays dockview's.
 */
function keepViewDocked(event: PointerEvent<HTMLElement>) {
  if (!event.shiftKey) return;
  const header = (event.target as HTMLElement).closest(".dv-tabs-and-actions-container");
  if (header && event.currentTarget.contains(header) && header.closest('[data-dock-group-kind="schau"]')) event.preventDefault();
}

/**
 * The deck of an area: the hinge on top, then the area's one dock (its own dockview with its own layout key): the view group in the middle,
 * the edit panels beside or below it as the arrangement puts them; every group shows its tabs, a panel can be dragged, floated, maximized.
 * Every area with a deck has its own `Deck` (the shell keeps them all mounted). While a wizard is open (world only), its step shows in a
 * strip under the dock (as high as the old hand, `HAND_DEFAULT`), which stays mounted and shrinks.
 */
export function Deck({ area, components, dockRef, hinge, wizard }: DeckProps) {
  return (
    <div className={styles.deck}>
      {hinge}
      <div className={styles.dock} id={`dock-${area}`} data-dock={area} onKeyDown={openTabMenu} onPointerDownCapture={keepViewDocked}>
        <DeckDock key={area} area={area} components={components} apiRef={dockRef} />
      </div>
      {wizard !== undefined && <div className={styles.wizard} data-wizard-hand="" hidden={!wizard} />}
    </div>
  );
}
