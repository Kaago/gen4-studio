import { useCallback, useContext, useEffect, useRef, useState } from "react";
import type { PreviewDto } from "../api";
import { StartServiceContext } from "./startService";

/** Joins paths into one dependency (a path holds no NUL). */
const SEP = "\0";

/**
 * The preview pictures of the recent projects (phase 4.0 design, §3.5): reads `project_preview_get { path }` for each path not in
 * `missing`, once while `visible` stays true and again each time it turns true. Only answers with a picture are entries; an answer
 * without one, or an error, means no picture (no message). `reload` reads one path again (after its picture was set or removed).
 */
export function usePreviews(
  paths: readonly string[],
  missing: ReadonlySet<string>,
  visible: boolean,
): { previews: ReadonlyMap<string, PreviewDto>; reload: (path: string) => void } {
  const { projectPreviewGet } = useContext(StartServiceContext);
  const [previews, setPreviews] = useState<ReadonlyMap<string, PreviewDto>>(() => new Map());
  // The paths read since Start last became visible, and the newest request per path (an older answer that comes late is dropped).
  const loaded = useRef(new Set<string>());
  const tickets = useRef(new Map<string, number>());
  const lastTicket = useRef(0);
  const wasVisible = useRef(false);

  const put = useCallback((path: string, dto: PreviewDto | null) => {
    setPreviews((before) => {
      if (dto === null && !before.has(path)) return before;
      const next = new Map(before);
      if (dto === null) next.delete(path);
      else next.set(path, dto);
      return next;
    });
  }, []);

  const load = useCallback(
    (path: string) => {
      const ticket = ++lastTicket.current;
      tickets.current.set(path, ticket);
      const current = () => tickets.current.get(path) === ticket;
      // Any failure (a refusal, a bridge that does not answer, a malformed answer) means no picture, without a message.
      Promise.resolve()
        .then(() => projectPreviewGet(path))
        .then((dto) => (dto?.source != null && dto.png != null ? dto : null))
        .catch(() => null)
        .then((dto) => {
          if (current()) put(path, dto);
        });
    },
    [projectPreviewGet, put],
  );

  const key = paths.filter((p) => !missing.has(p)).join(SEP);
  useEffect(() => {
    if (!visible) {
      wasVisible.current = false;
      return;
    }
    if (!wasVisible.current) {
      wasVisible.current = true;
      loaded.current.clear();
    }
    const wanted = new Set(key === "" ? [] : key.split(SEP));
    // A path that left the list (or went missing) is read again when it comes back; its picture goes now, and a late answer for it too.
    for (const path of [...loaded.current]) if (!wanted.has(path)) loaded.current.delete(path);
    for (const path of [...tickets.current.keys()]) if (!wanted.has(path)) tickets.current.delete(path);
    setPreviews((before) => ([...before.keys()].every((path) => wanted.has(path)) ? before : new Map([...before].filter(([path]) => wanted.has(path)))));
    for (const path of wanted) {
      if (loaded.current.has(path)) continue;
      loaded.current.add(path);
      load(path);
    }
  }, [visible, key, load]);

  const reload = useCallback(
    (path: string) => {
      loaded.current.add(path);
      load(path);
    },
    [load],
  );
  return { previews, reload };
}
