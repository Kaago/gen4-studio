import { useCallback, useEffect, useState } from "react";
import { applyHeaderPatch, errorText, headerPatchStatus, repairHeaderPatch } from "../../api";
import type { HeaderPatchDto } from "../../api";
import { useStableCallback } from "../../app/useStable";

/**
 * The Dynamic Headers patch as a wizard's "Header" step needs it: its state, read when the wizard opens, and the
 * action that applies it. `patch` stays null while it is being read and when reading it failed (then `error` says why:
 * the step offers no new header). `applied` runs after the patch went in (the app reads the header table again);
 * a refusal (a header table changed on disk) or a failure of either becomes `error`, the user fixes it and tries again.
 * Unsaved header edits need no saving first: the kernel patches with the table as it is on disk and keeps the edits
 * (and their undo) unsaved. `repair` fixes a patch whose routine calls a wrong function (`patch.repairable`); the
 * header table stays as it is. `repaired` runs after a repair went in (the app closes the toast that offered it and
 * says what changed).
 */
export function useHeaderPatch(applied: () => Promise<void>, { repaired }: { repaired?: () => void } = {}) {
  const [patch, setPatch] = useState<HeaderPatchDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const afterApply = useStableCallback(applied);

  useEffect(() => {
    let live = true;
    headerPatchStatus().then(
      (status) => live && setPatch(status),
      (e) => live && setError(errorText(e)),
    );
    return () => {
      live = false;
    };
  }, []);

  const apply = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setPatch(await applyHeaderPatch());
      await afterApply();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }, [afterApply]);

  const repair = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setPatch(await repairHeaderPatch());
      repaired?.();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }, [repaired]);

  return { patch, error, busy, apply, repair };
}
