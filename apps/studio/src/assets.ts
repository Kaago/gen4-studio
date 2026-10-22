import type { AssetId, AssetKind } from "./api";

const KIND_LABELS: Record<AssetKind, string> = {
  Map: "Karte",
  Matrix: "Matrix",
  AreaData: "Area-Data",
  Event: "Event-Datei",
  TextArchive: "Textarchiv",
  MapTexture: "Karten-Textur",
  BuildingTexture: "Gebäude-Textur",
  BuildingModel: "Gebäudemodell",
  InteriorBuildingModel: "Innen-Gebäudemodell",
  HeaderTable: "Header-Tabelle",
  Script: "Scriptdatei",
};

/** German display name of an asset kind. */
export function kindLabel(kind: AssetKind): string {
  return KIND_LABELS[kind];
}

/** "Karte 0000": German kind name and four-digit member index. */
export function assetLabel(asset: AssetId): string {
  return `${kindLabel(asset.kind)} ${String(asset.index).padStart(4, "0")}`;
}

/** Message of the bar when an asset changed on disk behind the editor's back. */
export function staleText(asset: AssetId): string {
  return `${assetLabel(asset)} wurde außerhalb geändert.`;
}
