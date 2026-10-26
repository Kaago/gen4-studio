import { useCallback, useEffect, useRef, useState } from "react";
import { projectPreviewGet } from "../api";
import type { PreviewDto } from "../api";
import { changePreview } from "./previewMessages";

export type ProjectPreviewState = {
  /** The answer of the last read or write; null until the first read answers. A failing read counts as no picture. */
  preview: PreviewDto | null;
  /** A choose or remove runs. */
  busy: boolean;
  /** „Vorschaubild wählen…“ or „Entfernen“ (with a toast, see `changePreview`); the answer becomes the shown preview. */
  change: (how: "choose" | "remove") => void;
};

const NONE: PreviewDto = { source: null, png: null, standardMap: null };

/**
 * The open project's preview picture (phase 4.0 design, §3.5, Technik): read once without a path when mounted, so a fresh read each time
 * Technik shows the project. Start reads its cards again when it shows, so a picture set here is there the next time Start shows.
 */
export function useProjectPreview(): ProjectPreviewState {
  const [preview, setPreview] = useState<PreviewDto | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  // A write answered: a first read that comes after it is older and is dropped.
  const wrote = useRef(false);

  useEffect(() => {
    alive.current = true;
    Promise.resolve()
      .then(() => projectPreviewGet())
      .then((dto) => dto ?? NONE)
      .catch(() => NONE)
      .then((dto) => {
        if (alive.current && !wrote.current) setPreview(dto);
      });
    return () => {
      alive.current = false;
    };
  }, []);

  const change = useCallback((how: "choose" | "remove") => {
    setBusy(true);
    void changePreview(how).then((dto) => {
      if (!alive.current) return;
      setBusy(false);
      if (dto === null) return;
      wrote.current = true;
      setPreview((before) => ({ ...dto, standardMap: dto.standardMap ?? before?.standardMap ?? null }));
    });
  }, []);

  return { preview, busy, change };
}
