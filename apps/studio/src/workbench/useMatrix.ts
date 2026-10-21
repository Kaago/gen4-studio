import { useEffect, useRef, useState } from "react";
import { getMatrix } from "../api";
import type { MatrixDto } from "../api";

/**
 * Matrix `index` as the backend holds it (unsaved edits included), read again whenever `revision`
 * changes. Keeps showing the previous values of the same matrix while it reloads; null for a
 * different matrix than the one asked for, and after a failed read (reported to `onError`).
 */
export function useMatrix(index: number | null, revision: number, onError: (e: unknown) => void): MatrixDto | null {
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  const report = useRef(onError);
  report.current = onError;

  useEffect(() => {
    if (index === null) {
      setMatrix(null);
      return;
    }
    let cancelled = false;
    getMatrix(index).then(
      (next) => {
        if (!cancelled) setMatrix(next);
      },
      (e) => {
        if (cancelled) return;
        setMatrix(null);
        report.current(e);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [index, revision]);

  return matrix !== null && matrix.index === index ? matrix : null;
}
