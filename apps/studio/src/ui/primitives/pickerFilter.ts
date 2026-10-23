import type { ReactNode } from "react";
import type { Area } from "./Reference";

export interface PickerItem {
  id: string | number;
  label: string;
  /** Technical name or number, searched like the label and shown in mono after it. */
  tech?: string;
  area: Area;
  /** Drawn at the start of the row (a map thumbnail, a sprite). */
  preview?: ReactNode;
  /** Shown at the end of the row ("12 Events"). */
  count?: string;
  /** Items with the same group sit under one visible group label. */
  group?: string;
}

/** Whether `id` equals the digits typed, ignoring leading zeros ("0411" finds 411). */
function idMatches(id: string | number, digits: string): boolean {
  const text = String(id);
  return /^\d+$/.test(text) && Number(text) === Number(digits);
}

/**
 * Case-insensitive substring in `label` or `tech`; a purely numeric query also finds the `id`. Matches that start the
 * label or tech (or are the id) come before matches further inside; within each kind the input order is kept.
 */
export function filterItems<T extends PickerItem>(items: readonly T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...items];
  const numeric = /^\d+$/.test(q);
  const starts: T[] = [];
  const inside: T[] = [];
  for (const item of items) {
    const fields = [item.label.toLowerCase(), item.tech?.toLowerCase() ?? ""];
    if (fields.some((f) => f.startsWith(q)) || (numeric && idMatches(item.id, q))) starts.push(item);
    else if (fields.some((f) => f.includes(q))) inside.push(item);
  }
  return [...starts, ...inside];
}
