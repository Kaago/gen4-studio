const BLOCKED = 0x80;

/** Overlay color for one permission cell: collision wins, then terrain kind family. */
export function permissionColor(kind: number, collision: number): string {
  if ((collision & BLOCKED) !== 0) return "#d64545";
  if (kind >= 0x02 && kind <= 0x07) return "#4caf50";
  if (kind >= 0x10 && kind <= 0x1f) return "#3b82f6";
  return "#6b7280";
}
