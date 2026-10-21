import type { CSSProperties } from "react";
import { tileSummary } from "../world/hover";
import type { TileInfo } from "../world/hover";

/** What the inspector shows. The event variant is added with the event markers. */
export type InspectorModel =
  | { kind: "none" }
  | {
      kind: "tile";
      /** Map file index and the tile inside it (0..31). */
      map: number;
      x: number;
      y: number;
      /** Tile position in the whole matrix. */
      absX: number;
      absZ: number;
      info: TileInfo;
    };

const box: CSSProperties = { padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 14, color: "#4b5563" };

export function Inspector({ model }: { model: InspectorModel }) {
  switch (model.kind) {
    case "none":
      return <div style={box}>Nichts ausgewählt. Mit der Maus über ein Tile fahren oder einen Marker anklicken.</div>;
    case "tile":
      return (
        <div style={box}>
          <div style={{ color: "#111827" }}>{tileSummary(model.map, model.x, model.y, model.info)}</div>
          <div style={{ marginTop: 4, fontSize: 12 }}>
            Matrix-Position x {model.absX}, z {model.absZ} · Kollision 0x
            {model.info.collision.toString(16).toUpperCase().padStart(2, "0")}
            {model.info.class ? ` · Klasse ${model.info.class}` : ""}
          </div>
        </div>
      );
  }
}
