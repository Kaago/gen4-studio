import { useCallback, useMemo, useSyncExternalStore } from "react";
import type { DockviewApi } from "dockview-react";
import { progressText } from "../checks/messages";
import { notChecked } from "../checks/useChecks";
import type { ChecksFeed } from "../checks/useChecks";
import { Hinge } from "../ui/frame/Hinge";
import type { HingeProps } from "../ui/frame/Hinge";
import { toggleSchauMaximized } from "../ui/frame/dockRules";
import { isSchauPanel } from "../workbench/layout";
import type { ProblemsStore } from "../workbench/problems/problems";
import { germanHistory } from "./historyLabel";
import type { HistoryFeed } from "./useHistory";
import type { DockRef } from "./useSceneSync";

export type HingeHostProps = Omit<HingeProps, "problems" | "history" | "checking" | "unchecked" | "maximized" | "onMaximize"> & {
  problems: ProblemsStore;
  /** The undo history as App reads it once for every hinge (`useHistoryReader`). */
  history: HistoryFeed;
  /** Whether the checks run, and how far they got. */
  checks: ChecksFeed;
  /** The dock below the hinge: „Schau maximieren“ maximizes its view. Absent: no such button. */
  dock?: DockRef;
};

/** Whether the view of the dock `api` is maximized. */
function viewMaximized(api: DockviewApi | null): boolean {
  try {
    return api?.panels.some((p) => isSchauPanel(p.id) && p.api.isMaximized()) ?? false;
  } catch {
    // The dock is going away.
    return false;
  }
}

/**
 * Whether the view of `dock` is maximized, heard from the dock itself: a tab's double click, its menu, Esc, a link or an arrangement change
 * it as well as the hinge's button. The dock hands out its api after the hinge rendered, so the hinge listens for that first.
 */
function useViewMaximized(dock: DockRef | undefined): boolean {
  const subscribe = useCallback(
    (changed: () => void) => {
      if (!dock) return () => {};
      let heard: { dispose: () => void } | undefined;
      const attach = () => {
        heard?.dispose();
        heard = dock.current?.onDidMaximizedGroupChange(changed);
        changed();
      };
      attach();
      const stop = dock.listen(attach);
      return () => {
        stop();
        heard?.dispose();
      };
    },
    [dock],
  );
  return useSyncExternalStore(subscribe, () => viewMaximized(dock?.current ?? null));
}

/**
 * The hinge with its own state: the problem counts, the progress of the checks and the undo history are subscribed to here, so a change of
 * any reaches this bar and not the panels (they get none of it through `WorkbenchData`), nor App. Every deck has a hinge; they all read the
 * same history, which App reads from the backend once per change. The history labels are the kernel's English ones and are named in German
 * on the way to the bar, in the words of the depth. Over a dock it has „Schau maximieren“, pressed while the dock's view is maximized.
 */
export function HingeHost({ problems, history, checks, dock, ...rest }: HingeHostProps) {
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  const view = useSyncExternalStore(checks.subscribe, checks.get);
  const read = useSyncExternalStore(history.subscribe, history.get);
  // Bauen names the steps without "Raster", "Matrix" and "Header" (a switch of the depth names them anew).
  const labels = useMemo(() => germanHistory(read, rest.depth), [read, rest.depth]);
  const maximized = useViewMaximized(dock);
  const onMaximize = useCallback(() => {
    const api = dock?.current;
    if (api) toggleSchauMaximized(api);
  }, [dock]);
  return (
    <Hinge
      {...rest}
      problems={counts}
      checking={view.running ? progressText(view.progress) : null}
      unchecked={notChecked(view)}
      history={labels}
      maximized={maximized}
      onMaximize={dock ? onMaximize : undefined}
    />
  );
}
