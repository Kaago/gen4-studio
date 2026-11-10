import { useEffect, useState } from "react";
import { t } from "../../i18n";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * "vor 4 Minuten" / "4 minutes ago", "vor 2 Stunden" / "2 hours ago", "gerade eben" / "just now" for less than a minute (and for a time in
 * the future), in the language that is shown. A component that shows it also calls `useTranslation`, so it renders again when the language changes.
 */
export function ago(from: number, now: number): string {
  const elapsed = now - from;
  if (elapsed < MINUTE) return t("ui:timeAgo.justNow");
  if (elapsed < HOUR) return t("ui:timeAgo.minutes", { count: Math.floor(elapsed / MINUTE) });
  if (elapsed < DAY) return t("ui:timeAgo.hours", { count: Math.floor(elapsed / HOUR) });
  return t("ui:timeAgo.days", { count: Math.floor(elapsed / DAY) });
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
