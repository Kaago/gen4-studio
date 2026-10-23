import { useMemo, useSyncExternalStore } from "react";
import { Hinge } from "../ui/frame/Hinge";
import type { HingeProps } from "../ui/frame/Hinge";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";
import { germanHistory } from "./historyLabel";
import { useHistory } from "./useHistory";

export type HingeHostProps = Omit<HingeProps, "problems" | "history"> & {
  bus: EditBus;
  problems: ProblemsStore;
  /** Changes after every undo, redo and write; the history is read again then. */
  historyRevision: number;
  /** Whether a project is open (without one there is no history). */
  hasProject: boolean;
};

/**
 * The hinge with its own state: the problem counts and the undo history are read here, so a change of either reaches this bar and not
 * the panels (they get none of it through `WorkbenchData`). App itself still renders per write (its dirty count moves); this host does
 * not keep App out of that. The history labels are the kernel's English ones and are named in German on the way to the bar.
 */
export function HingeHost({ bus, problems, historyRevision, hasProject, ...rest }: HingeHostProps) {
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  const read = useHistory(bus, historyRevision, hasProject);
  const history = useMemo(() => germanHistory(read), [read]);
  return <Hinge {...rest} problems={counts} history={history} />;
}
