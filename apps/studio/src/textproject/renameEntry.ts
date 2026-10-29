import type { MenuItemSpec } from "../ui/primitives";

/** The entry „ID umbenennen…“ of a head's menu „Aktionen“ (trainers and headers in a text project). */
export function renameEntry(id: string, open: (id: string) => void): MenuItemSpec {
  return { label: "ID umbenennen…", onSelect: () => open(id) };
}
