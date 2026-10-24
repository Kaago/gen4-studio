import { useCallback, useEffect, useState } from "react";
import { applyHeaderPatch, dirty, errorText, headerPatchStatus, repairHeaderPatch } from "../../api";
import type { HeaderPatchDto } from "../../api";
import { useStableCallback } from "../../app/useStable";

/** The way out when the header table is unsaved: in Technik one saves; a wizard locks ⌘S, so it is left first. */
export const SAVE_FIRST = "Erst speichern (⌘S), dann den Patch anwenden.";
export const SAVE_FIRST_IN_WIZARD = "Assistent abbrechen, speichern (⌘S) und neu öffnen.";

/**
 * The Dynamic Headers patch as a wizard's "Header" step needs it: its state, read when the wizard opens, and the
 * action that applies it. `patch` stays null while it is being read and when reading it failed (then `error` says why:
 * the step offers no new header). `applied` runs after the patch went in (the app reads the header table again);
 * a refusal (unsaved header edits) or a failure of either becomes `error`, the user fixes it and tries again. `repair`
 * fixes a patch whose routine calls a wrong function (`patch.repairable`); the header table stays as it is.
 * The patch takes the header table from disk, so unsaved header edits are caught before the call: `error` then
 * says so in German and ends with `saveFirst`, the way out where the hook is used (a wizard locks ⌘S). `repaired` runs after a repair
 * went in (the app closes the toast that offered it and says what changed).
 */
export function useHeaderPatch(applied: () => Promise<void>, { saveFirst = SAVE_FIRST, repaired }: { saveFirst?: string; repaired?: () => void } = {}) {
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
      // When the dirty list cannot be read, the kernel's own refusal still guards the patch.
      const unsaved = await dirty().catch(() => []);
      if (unsaved.some((asset) => asset.kind === "HeaderTable")) {
        setError(`Die Header-Tabelle hat ungespeicherte Änderungen; der Patch nimmt die Header von der Platte. ${saveFirst}`);
        return;
      }
      setPatch(await applyHeaderPatch());
      await afterApply();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }, [afterApply, saveFirst]);

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
