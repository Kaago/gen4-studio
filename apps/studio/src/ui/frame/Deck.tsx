import { memo, useEffect, useMemo, useRef, useSyncExternalStore } from "react";
import type { FunctionComponent, KeyboardEvent, MutableRefObject, PointerEvent, ReactNode } from "react";
import { DockviewDefaultTab, DockviewReact } from "dockview-react";
import type { DockviewApi, DockviewGroupPanel, DockviewReadyEvent, DockviewTheme, GetTabContextMenuItemsParams, IDockviewPanelHeaderProps, IDockviewPanelProps, SerializedDockview } from "dockview-react";
import { AREA_PANELS, DEFAULT_ARRANGEMENT, dropLayout, isSaneLayout, layoutKey, loadArrangement, loadLayout, saveArrangement, saveLayout } from "../../workbench/layout";
import type { Arrangement, DeckArea } from "../../workbench/layout";
import { scriptDirtyStore } from "../../workbench/script/scriptDirtyStore";
import { messages } from "./dockMessages";
import { tabMenuItems } from "./dockMenu";
import type { TabMenuItem } from "./dockMenu";
import { workMenuByKeyboard } from "./dockMenuKeys";
import { applyArrangement, dockPanel, fillWhileMaximized, floatPanel, isSchauGroup, keepViewGroupClosed, keepViewGroupInPlace, keepViewPanelsInPlace, restoreLimits, syncDepthPanels } from "./dockRules";
import type { Depth } from "./shellState";
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
  /** The depth of the shell: the dock has the panels of that depth (the Raster only in Tüfteln) and follows a change of it. */
  depth: Depth;
  /**
   * The sub-area the deck shows (Pokémon: Pokédex, Trainer, Begegnungen, Attacken): the dock keeps a layout per sub-area and swaps it when
   * this changes, without being built anew. Undefined: one layout for the area.
   */
  sub?: string;
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
  /** „Als Vorschaubild“ in the tab menu of the views „3D“ and „Weltkarte“ (the world's deck). Undefined: the menu has none. */
  preview?: DeckPreview;
};

/** What the tab menu's „Als Vorschaubild“ does for a view panel, and why it is locked (null: it is not). */
export type DeckPreview = { set: (panelId: string) => void; locked: (panelId: string) => string | null };

/** Numbers the hidden reasons of locked lines (their ids). */
let reasons = 0;

/**
 * A locked line that says why (`hint`): as its tooltip for the mouse, and for a screen reader as its description, a hidden text that follows
 * it in the menu (`aria-describedby`). dockview builds a line from a label without either, so this one is built here as dockview builds its
 * locked lines (same classes and roles: the keyboard reaches it, Enter does nothing, a click does nothing).
 */
function lockedLine(label: string, hint: string): HTMLElement[] {
  const reason = document.createElement("span");
  reason.id = `dock-menu-reason-${++reasons}`;
  reason.hidden = true;
  reason.textContent = hint;
  const el = document.createElement("div");
  el.className = "dv-context-menu-item dv-context-menu-item--disabled";
  el.setAttribute("role", "menuitem");
  el.setAttribute("aria-disabled", "true");
  el.setAttribute("aria-describedby", reason.id);
  el.title = hint;
  el.textContent = label;
  return [el, reason];
}

/** The tab menu's items as dockview takes them: a locked line with a reason as elements of its own (the line and its hidden reason). */
const dockviewItems = (items: TabMenuItem[]): (TabMenuItem | { element: HTMLElement })[] =>
  items.flatMap((item): (TabMenuItem | { element: HTMLElement })[] => (item !== "separator" && item.disabled && item.hint !== undefined ? lockedLine(item.label, item.hint).map((element) => ({ element })) : [item]));

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

/** Builds `arrangement` of `depth` in the dock and remembers it as the arrangement and layout of the area (its sub-area `sub`). */
function arrange(api: DockviewApi, area: DeckArea, arrangement: Arrangement, depth: Depth, sub: string | undefined) {
  applyArrangement(api, area, arrangement, depth);
  api.groups.forEach(markGroup);
  saveArrangement(area, arrangement, sub);
  saveLayout(layoutKey(area, sub), api.toJSON());
}

/**
 * Builds `arrangement` (undefined: the area's default) of `depth` in the dock of `area` and remembers it for the sub-area `sub` the dock
 * shows (undefined: an area without sub-areas). A dock that is not built yet (the world's, reset from an area without a deck before the
 * world was shown) forgets its stored layout and arrangement, so it starts from them when it is built.
 */
export function resetLayout(area: DeckArea, dockRef: MutableRefObject<DockviewApi | null>, arrangement: Arrangement | undefined, depth: Depth, sub?: string): void {
  const chosen = arrangement ?? DEFAULT_ARRANGEMENT[area];
  const api = dockRef.current;
  if (!api) {
    dropLayout(layoutKey(area, sub));
    saveArrangement(area, chosen, sub);
    return;
  }
  arrange(api, area, chosen, depth, sub);
}

/**
 * Restores the stored layout of the dock (of its sub-area `sub`) when it is complete (and no view panel floats in it), otherwise builds the
 * arrangement of `depth`. A restored layout (stored in either depth) gets the panels of `depth` afterwards (`syncDepthPanels`). The width
 * limits of a group are not stored: a panel whose step has them gets them again when it sits alone in a docked group (`restoreLimits`).
 * Also swaps the layout of a dock that is built (another sub-area): `fromJSON` and the arrangement replace what the dock holds.
 */
function initLayout(api: DockviewApi, area: DeckArea, depth: Depth, sub: string | undefined) {
  const saved = loadLayout(layoutKey(area, sub));
  if (saved && isSaneLayout(saved, AREA_PANELS[area])) {
    try {
      api.fromJSON(saved as SerializedDockview);
      syncDepthPanels(api, area, depth);
      restoreLimits(api, area, loadArrangement(area, sub));
      return;
    } catch {
      /* damaged layout: fall through to the arrangement */
    }
  }
  // The first build (or one after an unusable layout) remembers its arrangement, as a chosen one is.
  const arrangement = loadArrangement(area, sub);
  applyArrangement(api, area, arrangement, depth);
  saveArrangement(area, arrangement, sub);
}

/** The dock, memoised: the providers above it change with every hover, the dock itself must not. */
const DeckDock = memo(function DeckDock({
  area,
  depth,
  sub,
  components,
  apiRef,
  previewRef,
}: {
  area: DeckArea;
  depth: Depth;
  sub: string | undefined;
  components: DockComponents;
  apiRef: MutableRefObject<DockviewApi | null>;
  /** The deck's `preview`, read when the menu opens (a ref, so the memoised dock does not render for it). */
  previewRef: MutableRefObject<DeckPreview | undefined>;
}) {
  const panels = useMemo(() => withPanelAttrs(components), [components]);
  // The depth when the dock gets ready (dockview calls `onReady` once, from its first render) and when its tab menu resets it.
  const depthRef = useRef(depth);
  depthRef.current = depth;
  // The sub-area whose layout the dock holds: what it restores when it gets ready, and where it stores its changes.
  const subRef = useRef(sub);
  // A change of depth adds or removes the panels only for Tüfteln (the Raster); the dock is not built anew.
  useEffect(() => {
    const api = apiRef.current;
    if (api) syncDepthPanels(api, area, depth);
  }, [apiRef, area, depth]);
  // Another sub-area: the layout of the one shown so far is stored first (with its floating groups, before dockview announces its last
  // change), then the new one's is restored in the same dock. A dock that is not ready restores the new one when it gets ready.
  useEffect(() => {
    const api = apiRef.current;
    if (subRef.current === sub) return;
    if (api) {
      const json = api.toJSON();
      if (isSaneLayout(json, AREA_PANELS[area])) saveLayout(layoutKey(area, subRef.current), json);
    }
    subRef.current = sub;
    if (!api) return;
    initLayout(api, area, depthRef.current, sub);
    api.groups.forEach(markGroup);
  }, [apiRef, area, sub]);
  // The tab menu works on this dock; it is asked for its items on every right click.
  const menuItems = useMemo(
    () => (params: GetTabContextMenuItemsParams) => {
      const preview = previewRef.current;
      return dockviewItems(
        tabMenuItems(params, {
          float: (id) => void floatPanel(params.api, id),
          dock: (id) => void dockPanel(params.api, area, id, loadArrangement(area, subRef.current)),
          reset: () => resetLayout(area, apiRef, loadArrangement(area, subRef.current), depthRef.current, subRef.current),
          ...(preview === undefined ? {} : { setPreview: preview.set, previewLocked: preview.locked }),
        }),
      );
    },
    [area, apiRef, previewRef],
  );
  function onReady(event: DockviewReadyEvent) {
    const api = event.api;
    apiRef.current = api;
    initLayout(api, area, depthRef.current, subRef.current);
    api.groups.forEach(markGroup);
    // Intermediate states (clear, half-built arrangement) must not overwrite the user's layout. Stored under the sub-area shown now.
    const remember = () => {
      const json = api.toJSON();
      if (isSaneLayout(json, AREA_PANELS[area])) saveLayout(layoutKey(area, subRef.current), json);
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
export function Deck({ area, depth, sub, components, dockRef, hinge, wizard, preview }: DeckProps) {
  const previewRef = useRef(preview);
  previewRef.current = preview;
  return (
    <div className={styles.deck}>
      {hinge}
      <div className={styles.dock} id={`dock-${area}`} data-dock={area} onKeyDown={openTabMenu} onPointerDownCapture={keepViewDocked}>
        <DeckDock key={area} area={area} depth={depth} sub={sub} components={components} apiRef={dockRef} previewRef={previewRef} />
      </div>
      {wizard !== undefined && <div className={styles.wizard} data-wizard-hand="" hidden={!wizard} />}
    </div>
  );
}
