import { useEffect, useRef, useState } from "react";
import { diffEntity, errorText } from "../api";
import type { Against, DiffSubject, EntityDiffReport } from "../api";

/** What was read, for the subject and comparison it was read for. */
type Found = { key: string | null; report: EntityDiffReport | null; error: string | null; busy: boolean };

/**
 * The detail of Technik › Änderungen: the diff of `subject` against `against`, read when the subject or the comparison changes and again
 * when `revision` moves (a save, a reset, an edit). The same subject as a new object reads nothing again. While another subject is read
 * nothing of the one before shows; while the same one is read again its last diff stays. An answer for an older selection is dropped.
 */
export function useEntityDiff(subject: DiffSubject | null, against: Against, revision: number): { report: EntityDiffReport | null; error: string | null; busy: boolean } {
  const key = subject === null ? null : `${against}\u0000${JSON.stringify(subject)}`;
  const [found, setFound] = useState<Found>({ key: null, report: null, error: null, busy: false });
  // Bumped by every read: only the last one's answer counts.
  const serial = useRef(0);
  // The subject behind `key`, read in the effect (the effect follows the key, not the object).
  const latest = useRef(subject);
  latest.current = subject;

  useEffect(() => {
    const mine = ++serial.current;
    const wanted = latest.current;
    if (key === null || wanted === null) {
      setFound({ key: null, report: null, error: null, busy: false });
      return;
    }
    setFound((before) => (before.key === key ? { ...before, busy: true } : { key, report: null, error: null, busy: true }));
    diffEntity(wanted, against).then(
      (report) => mine === serial.current && setFound({ key, report, error: null, busy: false }),
      (e: unknown) => mine === serial.current && setFound({ key, report: null, error: errorText(e), busy: false }),
    );
  }, [key, against, revision]);

  // Before the effect of a new key has run, nothing of the old one shows.
  if (key === null) return { report: null, error: null, busy: false };
  if (found.key !== key) return { report: null, error: null, busy: true };
  return { report: found.report, error: found.error, busy: found.busy };
}
