import { useEffect, useRef, useState } from "react";
import { getEvents } from "../api";
import type { EventsDto } from "../api";

/**
 * The events of event file `file` (with `locked`, why it cannot be edited) as the backend holds them, read again whenever
 * `revision` changes. Keeps showing the previous values of the same file while it reloads; null while they load, for no
 * file, for another file than the one asked for, and after a failed read (reported to `onError`).
 */
export function useEvents(file: number | null, onError: (e: unknown) => void, revision = 0): EventsDto | null {
  const [events, setEvents] = useState<EventsDto | null>(null);
  const report = useRef(onError);
  report.current = onError;

  useEffect(() => {
    if (file === null) {
      setEvents(null);
      return;
    }
    let cancelled = false;
    getEvents(file).then(
      (next) => {
        if (!cancelled) setEvents(next);
      },
      (e) => {
        if (cancelled) return;
        setEvents(null);
        report.current(e);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [file, revision]);

  return events !== null && events.event_file_id === file ? events : null;
}
