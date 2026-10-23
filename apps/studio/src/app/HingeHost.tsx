import { useSyncExternalStore } from "react";
import { Hinge } from "../ui/frame/Hinge";
import type { HingeProps } from "../ui/frame/Hinge";
import type { EditBus } from "../bus/editBus";
import type { ProblemsStore } from "../workbench/problems/problems";
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
 * The hinge with its own state: the problem counts and the undo history are read here, so a change of either renders this bar and not
 * App (let alone the panels, which get none of it through `WorkbenchData`).
 */
export function HingeHost({ bus, problems, historyRevision, hasProject, ...rest }: HingeHostProps) {
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  const history = useHistory(bus, historyRevision, hasProject);
  return <Hinge {...rest} problems={counts} history={history} />;
}
