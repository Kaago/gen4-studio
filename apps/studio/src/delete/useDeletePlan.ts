import { useEffect, useState } from "react";
import { planDelete, resetErrorText } from "../api";
import type { DeletePlan, DeleteTarget } from "../api";

/** The plan of „Löschen…“ for `target`; `error` is the German reason when it could not be made. Both null while it is asked for. */
export interface DeletePlanState {
  plan: DeletePlan | null;
  error: string | null;
}

/**
 * Asks the kernel what deleting `target` would do (changes nothing), once per target (an equal target in a new object asks nothing); an
 * answer for a target before is dropped.
 */
export function useDeletePlan(target: DeleteTarget): DeletePlanState {
  const key = JSON.stringify(target);
  const [state, setState] = useState<DeletePlanState & { key: string }>({ plan: null, error: null, key });
  useEffect(() => {
    let alive = true;
    planDelete(JSON.parse(key) as DeleteTarget).then(
      (plan) => alive && setState({ plan, error: null, key }),
      (e: unknown) => alive && setState({ plan: null, error: resetErrorText(e), key }),
    );
    return () => {
      alive = false;
    };
  }, [key]);
  // Another target: nothing of the one before shows while its plan is asked for.
  return state.key === key ? { plan: state.plan, error: state.error } : { plan: null, error: null };
}
