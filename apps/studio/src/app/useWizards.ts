import { useCallback, useState } from "react";
import type { WizardRequest } from "../workbench/WorkbenchContext";
import { useStableCallback } from "./useStable";

interface WizardsParams {
  /**
   * The project changed under the wizard: read it again, and show `header` when it is not null;
   * `announcement` is the message once that is done. Called when a wizard was left after it wrote
   * (with null), and by `created` (with the new header).
   */
  onCreated: (header: number | null, announcement?: string) => Promise<void>;
}

/** Which wizard is open (if any) with what it is prefilled with. Only the app's wizard host reads `request`. */
export function useWizards({ onCreated }: WizardsParams) {
  const [request, setRequest] = useState<WizardRequest | null>(null);
  const refresh = useStableCallback(onCreated);

  /** Stable: it is handed to memoised panels as `onOpenWizard`. */
  const open = useCallback((next: WizardRequest) => setRequest(next), []);
  /** Leaves the wizard; `written` says it changed the project, which is then read again (nothing is shown). */
  const close = useCallback(
    (written = false) => {
      setRequest(null);
      if (written) void refresh(null);
    },
    [refresh],
  );
  /** "Im Studio zeigen": leaves the wizard, reads the project again and shows `header` (null: only reads), then announces. */
  const created = useCallback(
    (header: number | null, announcement?: string) => {
      setRequest(null);
      return refresh(header, announcement);
    },
    [refresh],
  );

  return { request, open, close, created };
}

export type Wizards = ReturnType<typeof useWizards>;
