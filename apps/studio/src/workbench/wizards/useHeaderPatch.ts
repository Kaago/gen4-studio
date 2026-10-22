import { useCallback, useEffect, useState } from "react";
import { applyHeaderPatch, errorText, headerPatchStatus } from "../../api";
import type { HeaderPatchDto } from "../../api";
import { useStableCallback } from "../../app/useStable";

/**
 * The Dynamic Headers patch as a wizard's "Header" step needs it: its state, read when the wizard opens, and the
 * action that applies it. `patch` stays null while it is being read and when reading it failed (then `error` says why:
 * the step offers no new header). `applied` runs after the patch went in (the app reads the header table again);
 * a refusal (unsaved header edits) or a failure of either becomes `error`, the user fixes it and tries again.
 */
export function useHeaderPatch(applied: () => Promise<void>) {
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

  return { patch, error, busy, apply };
}
