const BLOCKED = 0x80;

const CLASS_COLORS: Record<string, string> = {
  grass: "#4caf50",
  water: "#3b82f6",
  ice: "#67e8f9",
  sand: "#e0c068",
  ledge: "#a855f7",
  block: "#b45309",
  slide: "#ec4899",
  warp: "#f59e0b",
  door: "#f59e0b",
  bridge: "#92400e",
  furniture: "#14b8a6",
};

/** Walkable cells without a behaviour class colour (unnamed or `other`). */
const NEUTRAL = "#6b7280";

/**
 * Overlay color for one permission cell: collision wins, then the behaviour class of the
 * terrain kind (null when the kind has no name). The raw `kind` byte no longer matters: the
 * class carries its meaning.
 */
export function permissionColor(_kind: number, collision: number, cls: string | null): string {
  if ((collision & BLOCKED) !== 0) return "#d64545";
  return (cls !== null && CLASS_COLORS[cls]) || NEUTRAL;
}
