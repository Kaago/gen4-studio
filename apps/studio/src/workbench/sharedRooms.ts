/** „Geteilt mit 17 Räumen“: the badge of a room whose maps `shared` other rooms use too. */
export function sharedText(shared: number): string {
  return `Geteilt mit ${shared} ${shared === 1 ? "Raum" : "Räumen"}`;
}

/** What the badge „Geteilt“ explains on hover. */
export const SHARED_HINT = "Diese Karten gehören auch anderen Räumen. Was du hier änderst, ändert sie alle.";
