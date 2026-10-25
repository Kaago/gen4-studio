import type { Box, DockviewApi, DockviewGroupPanel, DockviewWillShowOverlayLocationEvent, GroupDragEvent, IDockviewPanel, Position, TabDragEvent } from "dockview-react";
import { AREA_PANELS, arrangementSteps, isSchauPanel, loadArrangement } from "../../workbench/layout";
import type { Arrangement, ArrangementStep, DeckArea, PanelId } from "../../workbench/layout";
import { messages } from "./dockMessages";

/** A floating panel's group: as wide as the inspector beside the view, high enough for its head and a few fields (px). */
const FLOAT_WIDTH = 340;
const FLOAT_HEIGHT = 480;
/** How far a floating group keeps from the view's edges (px). */
const FLOAT_INSET = 16;

/** Where a step's direction drops a panel: beside a group, or into it as a tab. */
const POSITION: Record<NonNullable<ArrangementStep["position"]>["direction"], Position> = {
  within: "center",
  right: "right",
  left: "left",
  below: "bottom",
  above: "top",
};

/** True for the group that holds the view panels (3D, Raster, Weltkarte; the previews). */
export const isSchauGroup = (group: DockviewGroupPanel): boolean => group.panels.some((p) => isSchauPanel(p.id));

/** `onWillDragPanel`: a view panel stays in the view group (its tab can still be clicked, it just cannot be dragged away). */
export function keepViewPanelsInPlace(event: TabDragEvent): void {
  if (isSchauPanel(event.panel.id)) event.nativeEvent.preventDefault();
}

/** `onWillDragGroup`: nor can the view group be dragged by its header, which would move it or let it float. */
export function keepViewGroupInPlace(event: GroupDragEvent): void {
  if (isSchauGroup(event.group)) event.nativeEvent.preventDefault();
}

/** `onWillShowOverlay`: nothing drops into the view group as a tab (its tabs, its header, its middle); a split beside it is fine. */
export function keepViewGroupClosed(event: DockviewWillShowOverlayLocationEvent): void {
  const group = event.group;
  if (!group || !isSchauGroup(group)) return;
  if (event.kind === "tab" || event.kind === "header_space" || event.position === "center") event.preventDefault();
}

/** Gives a group the width limits of the step that placed it (the inspector beside the view keeps between 280 and 480 px). */
function limit(group: DockviewGroupPanel, step: ArrangementStep) {
  if (step.minimumWidth === undefined && step.maximumWidth === undefined) return;
  group.api.setConstraints({ minimumWidth: step.minimumWidth, maximumWidth: step.maximumWidth });
}

/** Brings a step's group to its size; only a group in the grid has one to set. */
function size(group: DockviewGroupPanel, step: ArrangementStep) {
  if (group.api.location.type !== "grid") return;
  if (step.width !== undefined) group.api.setSize({ width: step.width });
  if (step.height !== undefined) group.api.setSize({ height: step.height });
}

/** Builds the arrangement into an empty dock: dockview adds each panel where its step says. */
function build(api: DockviewApi, area: DeckArea, steps: ArrangementStep[]) {
  const titles = new Map<string, string>(AREA_PANELS[area].map((p) => [p.id, p.title]));
  api.clear();
  for (const step of steps) {
    const panel = api.addPanel({
      id: step.id,
      component: step.id,
      title: titles.get(step.id) ?? step.id,
      // Keep the DOM (and the three.js canvas) alive when a tab is hidden or the panel is moved.
      renderer: "always",
      position: step.position ? { referencePanel: step.position.referencePanel, direction: step.position.direction } : undefined,
      initialWidth: step.width,
      initialHeight: step.height,
    });
    limit(panel.group, step);
  }
}

/**
 * Rebuilds the arrangement from the panels the dock has, moving them instead of adding them again: the scene, the editors and their
 * documents stay as they are. All of them gather in the view group first (a floating one comes back into the grid), then each step that
 * opens a group of its own moves its panel out beside its reference, and the tabs follow into theirs.
 */
function rearrange(api: DockviewApi, steps: ArrangementStep[]) {
  const panel = (id: PanelId) => api.getPanel(id) as IDockviewPanel;
  const home = panel(steps[0]!.id).group;
  steps.forEach((step, index) => {
    if (index > 0) panel(step.id).api.moveTo({ group: home, position: "center", index });
  });
  for (const step of steps) {
    if (!step.position) continue;
    const moving = panel(step.id);
    const reference = panel(step.position.referencePanel).group;
    if (step.position.direction === "within") {
      if (moving.group !== reference) moving.api.moveTo({ group: reference, position: "center" });
      continue;
    }
    moving.api.moveTo({ group: reference, position: POSITION[step.position.direction] });
    size(moving.group, step);
    limit(moving.group, step);
  }
}

/**
 * Builds `arrangement` in the dock of `area`: rearranges the panels it has (they keep their state), or builds the dock from scratch when it
 * is not complete (the first build). A maximized group is restored first. Afterwards every group shows the panel its arrangement opens
 * with, and the view group is the active one, 3D in front: dockview would leave the panel added or moved last in front. Nothing is
 * announced while the panels move.
 */
export function applyArrangement(api: DockviewApi, area: DeckArea, arrangement: Arrangement): void {
  const steps = arrangementSteps(area, arrangement);
  if (api.hasMaximizedGroup()) api.exitMaximizedGroup();
  api.updateOptions({ announcements: false });
  try {
    const complete = api.panels.length === steps.length && steps.every((step) => api.getPanel(step.id) !== undefined);
    if (complete) rearrange(api, steps);
    else build(api, area, steps);
    // The panels that open a group, last first, so the first one (the view's) ends up the active panel.
    const openers = steps.filter((step) => step.position?.direction !== "within").reverse();
    for (const step of openers) api.getPanel(step.id)?.api.setActive();
  } finally {
    api.updateOptions({ announcements: undefined });
  }
}

/**
 * Lets the panel `id` float at the right edge of the view, inspector-sized; false for a view panel (it stays docked), a panel the dock does
 * not have and one that floats already. A maximized group is restored first (the floating group would show above it otherwise).
 */
export function floatPanel(api: DockviewApi, id: PanelId | string): boolean {
  const panel = api.getPanel(id);
  if (!panel || isSchauPanel(id) || panel.api.location.type !== "grid") return false;
  if (api.hasMaximizedGroup()) api.exitMaximizedGroup();
  const view = api.groups.find((g) => g.api.location.type === "grid" && isSchauGroup(g));
  // The group api measures where the group is in the dock (dockview has it, its typing does not).
  const box = view ? (view.api as { boundingBox?: Box }).boundingBox : undefined;
  const right = box ? box.left + box.width : api.width;
  const top = box ? box.top : 0;
  api.addFloatingGroup(panel, { x: Math.max(0, right - FLOAT_WIDTH - FLOAT_INSET), y: top + FLOAT_INSET, width: FLOAT_WIDTH, height: FLOAT_HEIGHT });
  return true;
}

/**
 * Says `message` in the dock's live region. dockview's component can (`announce`); its api does not pass it on, so it is reached through
 * the api. Without it nothing is said.
 */
function announce(api: DockviewApi, message: string) {
  (api as unknown as { component?: { announce?: (text: string) => void } }).component?.announce?.(message);
}

/**
 * Docks the floating panel `id` where `arrangement` (by default the one last built in the area) puts it: into the group of a panel that is
 * its tab or whose tab it is, when that one is docked (Doppeldeck: the inspector back to script and problems), otherwise beside its
 * reference with its step's size and limits (Werkbank: the inspector right of everything, in full height). False when it does not float.
 */
export function dockPanel(api: DockviewApi, area: DeckArea, id: PanelId | string, arrangement: Arrangement = loadArrangement(area)): boolean {
  const panel = api.getPanel(id);
  if (!panel || panel.api.location.type !== "floating") return false;
  const steps = arrangementSteps(area, arrangement);
  const step = steps.find((s) => s.id === id);
  const docked = (other: string | undefined) => {
    const found = other === undefined ? undefined : api.getPanel(other);
    return found && found !== panel && found.api.location.type === "grid" ? found : undefined;
  };
  // A tab finds its group through the panel it is a tab of, or through a panel that is a tab of it.
  const mates = [step?.position?.direction === "within" ? step.position.referencePanel : undefined, ...steps.filter((s) => s.position?.direction === "within" && s.position.referencePanel === id).map((s) => s.id)];
  const mate = mates.map(docked).find((p) => p !== undefined);
  if (api.hasMaximizedGroup()) api.exitMaximizedGroup();
  const reference = docked(step?.position?.referencePanel) ?? api.panels.find((p) => isSchauPanel(p.id));
  if (mate) {
    panel.api.moveTo({ group: mate.group, position: "center" });
  } else if (!step?.position || !reference) {
    // No place in the arrangement (or nothing docked to put it beside): right of everything.
    panel.api.moveTo({ group: api.addGroup({ direction: "right" }), position: "center" });
  } else {
    const direction = step.position.direction === "within" ? "below" : step.position.direction;
    // Beside the view to the left or right means in full height: a group at that edge of the whole dock, not of the view's column.
    const group = direction === "right" || direction === "left" ? api.addGroup({ direction }) : api.addGroup({ referenceGroup: reference.group, direction });
    panel.api.moveTo({ group, position: "center" });
    size(panel.group, step);
    limit(panel.group, step);
  }
  // The panel moved out of its floating group (which closes), so dockview has no change of place to announce.
  announce(api, messages.groupDocked!(panel.title ?? panel.id));
  return true;
}

/**
 * „Schau maximieren“: maximizes the view group, or restores it when it is maximized; true when the view is maximized afterwards. Another
 * maximized group gives way to the view.
 */
export function toggleSchauMaximized(api: DockviewApi): boolean {
  const view = api.panels.find((p) => isSchauPanel(p.id) && p.api.location.type === "grid");
  if (!view) return false;
  if (view.api.isMaximized()) {
    view.api.exitMaximized();
    return false;
  }
  if (api.hasMaximizedGroup()) api.exitMaximizedGroup();
  api.maximizeGroup(view.group.activePanel ?? view);
  return true;
}
