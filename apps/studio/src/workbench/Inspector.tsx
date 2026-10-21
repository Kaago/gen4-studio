import type { CSSProperties } from "react";
import type { BgDto, CoordDto, ObjectDto, WarpDto } from "../api";
import { eventDetails, eventTitle } from "../world/events";
import { tileSummary } from "../world/hover";
import type { TileInfo } from "../world/hover";
import { HeaderEditor } from "./inspector/HeaderEditor";
import type { HeaderEditorContext } from "./inspector/HeaderEditor";

/** What the inspector shows. */
export type InspectorModel =
  | { kind: "none" }
  /** The selected header, shown as an editable form. */
  | { kind: "header"; id: number }
  | {
      kind: "event";
      eventKind: "warp" | "object" | "coord" | "bg";
      index: number;
      data: WarpDto | ObjectDto | CoordDto | BgDto;
      /** Warps: location name (or internal name) of the destination header. */
      destName?: string;
    }
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

export function Inspector({ model, headerEditor }: { model: InspectorModel; headerEditor: HeaderEditorContext }) {
  switch (model.kind) {
    case "header":
      return <HeaderEditor key={model.id} id={model.id} context={headerEditor} />;
    case "none":
      return <div style={box}>Nichts ausgewählt. Mit der Maus über ein Tile fahren oder einen Marker anklicken.</div>;
    case "event":
      return (
        <div style={box}>
          <div style={{ color: "#111827" }}>{eventTitle(model.eventKind, model.data, model.destName)}</div>
          <div style={{ marginTop: 4, fontSize: 12 }}>{eventDetails(model.eventKind, model.data)}</div>
        </div>
      );
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
