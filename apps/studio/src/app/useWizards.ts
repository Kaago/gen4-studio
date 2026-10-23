import { useCallback, useState } from "react";
import type { WizardRequest } from "../workbench/WorkbenchContext";
import { useStableCallback } from "./useStable";

/** What a wizard made that the app is to show: the matrix and the header that now has it (null: none was assigned), and the message to leave behind. */
export interface Created {
  matrix: number;
  header: number | null;
  announcement: string;
}

interface WizardsParams {
  /**
   * The project changed under the wizard: read it again and show `created`. Called when a wizard
   * was left after it wrote (with null: only read), and by `created`.
   */
  onCreated: (created: Created | null) => Promise<void>;
  /** A wizard opens: the app shows the world (its step bar is in the world's hinge) and puts the tool back on "Auswählen". */
  onOpen?: () => void;
}

/** Which wizard is open (if any) with what it is prefilled with. Only the app's wizard host reads `request`. */
export function useWizards({ onCreated, onOpen }: WizardsParams) {
  const [request, setRequest] = useState<WizardRequest | null>(null);
  const refresh = useStableCallback(onCreated);
  const opening = useStableCallback(() => onOpen?.());

  /** Stable: it is handed to memoised panels as `onOpenWizard`. */
  const open = useCallback(
    (next: WizardRequest) => {
      opening();
      setRequest(next);
    },
    [opening],
  );
  /** Leaves the wizard; `written` says it changed the project, which is then read again (nothing is shown). */
  const close = useCallback(
    (written = false) => {
      setRequest(null);
      if (written) void refresh(null);
    },
    [refresh],
  );
  /** "Zeigen": leaves the wizard, reads the project again and shows what it made. */
  const created = useCallback(
    (made: Created) => {
      setRequest(null);
      return refresh(made);
    },
    [refresh],
  );

  return { request, open, close, created };
}

export type Wizards = ReturnType<typeof useWizards>;
