/**
 * What Gen4 Studio remembers about updates, in `localStorage` (`g4.update.v1`): whether it may look for a new version on GitHub, when it did
 * last and which version the person skipped. Reading and writing never fail: a blocked, empty or broken storage gives the defaults (ask
 * once per start, nothing checked), and what cannot be saved is simply not remembered. Used by `useUpdates`.
 */

/** `ask`: the question is still to be asked; `auto`: look at startup (at most once a day); `off`: never on its own. */
export type UpdateConsent = "ask" | "auto" | "off";

/** `lastCheck` is an ISO 8601 time of the last successful check, `skipped` the version the person chose to skip. */
export type UpdatePrefs = { consent: UpdateConsent; lastCheck: string | null; skipped: string | null };

export const UPDATE_PREFS_KEY = "g4.update.v1";

/** At most one automatic check per day. */
export const CHECK_INTERVAL_MS = 86_400_000;

export const DEFAULT_UPDATE_PREFS: UpdatePrefs = { consent: "ask", lastCheck: null, skipped: null };

const isConsent = (v: unknown): v is UpdateConsent => v === "ask" || v === "auto" || v === "off";

/** The stored preferences; each field that is missing or of the wrong type is its default on its own. Never throws. */
export function readUpdatePrefs(storage?: Storage): UpdatePrefs {
  try {
    const raw = (storage ?? localStorage).getItem(UPDATE_PREFS_KEY);
    if (raw === null) return { ...DEFAULT_UPDATE_PREFS };
    const value: unknown = JSON.parse(raw);
    if (typeof value !== "object" || value === null || Array.isArray(value)) return { ...DEFAULT_UPDATE_PREFS };
    const { consent, lastCheck, skipped } = value as Record<string, unknown>;
    return {
      consent: isConsent(consent) ? consent : DEFAULT_UPDATE_PREFS.consent,
      lastCheck: typeof lastCheck === "string" ? lastCheck : DEFAULT_UPDATE_PREFS.lastCheck,
      skipped: typeof skipped === "string" ? skipped : DEFAULT_UPDATE_PREFS.skipped,
    };
  } catch {
    return { ...DEFAULT_UPDATE_PREFS };
  }
}

/** Saves the preferences; a storage that refuses (blocked, full) is ignored. */
export function writeUpdatePrefs(prefs: UpdatePrefs, storage?: Storage): void {
  try {
    (storage ?? localStorage).setItem(UPDATE_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    /* not remembered: the defaults apply next time */
  }
}

/**
 * Whether the startup check is due at `now` (milliseconds since the epoch): only with consent `auto`, and then when nothing was checked
 * yet, when the last check is no date or lies in the future (a clock set back) or when it is a full `CHECK_INTERVAL_MS` ago.
 */
export function isCheckDue(prefs: UpdatePrefs, now: number): boolean {
  if (prefs.consent !== "auto") return false;
  if (prefs.lastCheck === null) return true;
  const last = Date.parse(prefs.lastCheck);
  if (Number.isNaN(last) || last > now) return true;
  return now - last >= CHECK_INTERVAL_MS;
}
