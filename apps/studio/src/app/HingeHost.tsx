import { useMemo, useSyncExternalStore } from "react";
import { Hinge } from "../ui/frame/Hinge";
import type { HingeProps } from "../ui/frame/Hinge";
import type { ProblemsStore } from "../workbench/problems/problems";
import { germanHistory } from "./historyLabel";
import type { HistoryFeed } from "./useHistory";

export type HingeHostProps = Omit<HingeProps, "problems" | "history"> & {
  problems: ProblemsStore;
  /** The undo history as App reads it once for every hinge (`useHistoryReader`). */
  history: HistoryFeed;
};

/**
 * The hinge with its own state: the problem counts and the undo history are subscribed to here, so a change of either reaches this bar
 * and not the panels (they get none of it through `WorkbenchData`), nor App. Every deck has a hinge; they all read the same history,
 * which App reads from the backend once per change. The history labels are the kernel's English ones and are named in German on the
 * way to the bar.
 */
export function HingeHost({ problems, history, ...rest }: HingeHostProps) {
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  const read = useSyncExternalStore(history.subscribe, history.get);
  const labels = useMemo(() => germanHistory(read), [read]);
  return <Hinge {...rest} problems={counts} history={labels} />;
}
