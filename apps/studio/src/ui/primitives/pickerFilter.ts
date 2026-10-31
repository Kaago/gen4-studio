import type { ReactNode } from "react";
import type { IconName } from "./icons";
import type { Area } from "./Reference";

export interface PickerItem {
  id: string | number;
  label: string;
  /** Technical name or number, searched like the label and shown in mono after it. */
  tech?: string;
  /** The area the item belongs to: its symbol and colour mark the row. Commands have none and name an `icon` instead. */
  area?: Area;
  /** Symbol of the row when `area` is not set (or to override the area symbol). */
  icon?: IconName;
  /** Key combination shown at the end of the row (`Kbd`), e.g. "⌘S". */
  shortcut?: string;
  /** A second, grey line under the label; searched like the label. */
  description?: string;
  /** Drawn at the start of the row (a map thumbnail, a sprite). */
  preview?: ReactNode;
  /** Shown at the end of the row ("12 Events"). */
  count?: string;
  /** Items with the same group sit under one visible group label. */
  group?: string;
  /** Listed (and found) but not pickable: greyed, skipped by the arrow keys; the `description` can say why. */
  disabled?: boolean;
  /** Further words the search finds like the label but that are not shown (a second name, the number in another base). */
  keywords?: readonly string[];
}

/** Whether `id` equals the digits typed, ignoring leading zeros ("0411" finds 411). */
function idMatches(id: string | number, digits: string): boolean {
  const text = String(id);
  return /^\d+$/.test(text) && Number(text) === Number(digits);
}

/**
 * Case-insensitive substring in `label`, `tech`, `keywords` or `description`; a purely numeric query also finds the `id`. Matches
 * that start the label, tech or a keyword (or are the id) come before matches further inside (including every description match);
 * within each kind the input order is kept.
 */
export function filterItems<T extends PickerItem>(items: readonly T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...items];
  const numeric = /^\d+$/.test(q);
  const starts: T[] = [];
  const inside: T[] = [];
  for (const item of items) {
    const fields = [item.label.toLowerCase(), item.tech?.toLowerCase() ?? "", ...(item.keywords ?? []).map((k) => k.toLowerCase())];
    if (fields.some((f) => f.startsWith(q)) || (numeric && idMatches(item.id, q))) starts.push(item);
    else if (fields.some((f) => f.includes(q)) || item.description?.toLowerCase().includes(q)) inside.push(item);
  }
  return [...starts, ...inside];
}
