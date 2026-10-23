import { useEffect, useState } from "react";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const unit = (n: number, one: string, many: string) => `vor ${n} ${n === 1 ? one : many}`;

/** "vor 4 Minuten", "vor 2 Stunden", "gerade eben" for less than a minute (and for a time in the future). */
export function ago(from: number, now: number): string {
  const elapsed = now - from;
  if (elapsed < MINUTE) return "gerade eben";
  if (elapsed < HOUR) return unit(Math.floor(elapsed / MINUTE), "Minute", "Minuten");
  if (elapsed < DAY) return unit(Math.floor(elapsed / HOUR), "Stunde", "Stunden");
  return unit(Math.floor(elapsed / DAY), "Tag", "Tagen");
}

/** The current time at every render, and a render every `ms` (so "vor 4 Minuten" moves on while the window stays open). */
export function useNow(ms = 30_000): number {
  const [, tick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => tick((n) => n + 1), ms);
    return () => clearInterval(id);
  }, [ms]);
  return Date.now();
}
