import type { CSSProperties } from "react";

/** What the inspector shows. Tile and event variants are added with the tile hover and event markers. */
export type InspectorModel = { kind: "none" };

const box: CSSProperties = { padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 14, color: "#4b5563" };

export function Inspector({ model }: { model: InspectorModel }) {
  switch (model.kind) {
    case "none":
      return <div style={box}>Nichts ausgewählt. Mit der Maus über ein Tile fahren oder einen Marker anklicken.</div>;
  }
}
